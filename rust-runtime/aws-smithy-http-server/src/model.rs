pub struct HealthcheckInput;
pub struct HealthcheckOutput;
pub struct HealthcheckOperationInput(pub HealthcheckInput);
pub struct HealthcheckOperationOutput(pub HealthcheckOutput);

pub struct RegisterServiceInput;
pub struct RegisterServiceOutput;
pub struct RegisterServiceError;
pub struct RegisterServiceOperationInput(pub RegisterServiceInput);
pub struct RegisterServiceOperationOutput(pub RegisterServiceOutput);

impl From<HealthcheckOperationInput> for HealthcheckInput {
    fn from(v: HealthcheckOperationInput) -> Self {
        v.0
    }
}
