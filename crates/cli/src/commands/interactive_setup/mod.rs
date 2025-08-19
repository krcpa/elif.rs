pub mod interactive_config;
pub mod interactive_wizard;

use elif_core::ElifError;

pub use interactive_wizard::InteractiveSetupArgs;

/// Interactive project setup wizard
pub async fn run(args: InteractiveSetupArgs) -> Result<(), ElifError> {
    interactive_wizard::run_wizard(args).await
}