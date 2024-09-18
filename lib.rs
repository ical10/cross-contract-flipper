// If `std` feature is disabled, we are building for Wasm target
// and we need to use `no_std` and `no_main` attributes
// to compile the contract as a Wasm binary.
// If `std` feature is enabled, we are building for native target
// and we don't need these attributes.
// ink! builds in `std` mode when running tests.
//
// `no_std` attribute disables the standard library.
// When `no_std` is enabled, the `core` and `alloc` libraries are available.
// The `ink` crate provides necessary functionality in place of the standard library.
// `no_main` attribute disables the default entry point for the binary.
// We define our own entry point using the `#[ink::contract]` attribute.
#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// Docs on utilities to call or instantiate contracts on the chain:
/// https://docs.rs/ink_env/5.0.0/ink_env/call/index.html

#[ink::contract]
mod cross_contract_flipper {
    use ink::{
        env::{
            call::{build_call, ExecutionInput, Selector},
            CallFlags, DefaultEnvironment,
        },
        storage::{traits::ManualKey, Lazy, Mapping},
    };

    #[ink(storage)]
    pub struct CrossContractFlipper {
        value: bool,
        delegate_to: Lazy<Hash>,
    }

    impl CrossContractFlipper {
        /// Creates a new delegator smart contract with an initial value,
        /// and the code hash of the contract it will delegate to
        ///
        /// Additionally, this code hash will be locked to prevent its deletion
        /// because it is a dependency of this contract.
        #[ink(constructor)]
        pub fn new(init_value: bool, code_hash: Hash) -> Self {
            let mut delegate_to = Lazy::new();
            delegate_to.set(&code_hash);

            Self::env().lock_delegate_dependency(&code_hash);

            Self {
                value: init_value,
                delegate_to,
            }
        }

        // Call 'flip' method of the other contract using delegate call
        #[ink(message)]
        pub fn call_delegate_flip(&mut self) {
            let selector = ink::selector_bytes!("flip");
            let _ = build_call::<DefaultEnvironment>()
                .delegate(self.delegate_to())
                .call_flags(CallFlags::TAIL_CALL)
                .exec_input(ExecutionInput::new(Selector::new(selector)))
                .returns::<()>()
                .try_invoke();
        }

        fn delegate_to(&self) -> Hash {
            self.delegate_to
                .get()
                .expect("Delegate to always has a value")
        }

        /// Returns the current value in storage
        #[ink(message)]
        pub fn get(&self) -> bool {
            self.value
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::{ChainBackend, ContractsBackend};

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn e2e_flip_test<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
            let origin = client
                .create_and_fund_account(&ink_e2e::alice(), 10_000_000_000_000)
                .await;

            let code_hash = client
                .upload("other-contract", &origin)
                .submit()
                .await
                .expect("other_contract upload failed")
                .code_hash;

            let mut constructor = CrossContractFlipperRef::new(false, code_hash);
            let contract = client
                .instantiate("cross-contract-flipper", &origin, &mut constructor)
                .submit()
                .await
                .expect("cross-contract-flipper instantiate failed");
            let mut call_builder = contract.call_builder::<CrossContractFlipper>();

            let call_delegate_flip = call_builder.call_delegate_flip();

            let result = client.call(&origin, &call_delegate_flip).submit().await;
            assert!(result.is_ok(), "Calling `delegate_flip` failed");

            let expected_value = false;
            let call_builder = contract.call_builder::<CrossContractFlipper>();

            let call_get = call_builder.get();

            let call_get_result = client
                .call(&origin, &call_get)
                .submit()
                .await
                .unwrap()
                .return_value();

            assert_eq!(
                call_get_result, expected_value,
                "Expected value to be false"
            );

            Ok(())

            // let mut call_builder = contract.call_builder::<CrossContractFlipper>();
            // let call = call_builder.delegate_flip();

            // client
            //     .call(&ink_e2e::alice(), &call)
            //     .submit().await
            //     .expect("Calling `delegate_flip` failed");

            // let call = call_builder.call_get();

            // let result = client
            //     .call(&ink_e2e::alice(), &call)
            //     .submit().await
            //     .expect("Calling `call_get` failed")
            //     .return_value();

            // assert_eq!(result, false);

            // Ok(())
        }
    }
}
