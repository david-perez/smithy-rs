use simple::{error, input, output};

// Newtypes to impl `IntoResponse` and `FromRequest`.
// These won't be needed in reality because the traits will be code-generated in the `input` and
// `output` crates.
pub struct HealthcheckOutput(pub output::HealthcheckOutput);
pub struct HealthcheckInput(pub input::HealthcheckInput);
pub struct RegisterServiceOutput(pub output::RegisterServiceOutput);
pub struct RegisterServiceInput(pub input::RegisterServiceInput);
pub struct RegisterServiceError(pub error::RegisterServiceError);
