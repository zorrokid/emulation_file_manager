use crate::domains::system::effect::SystemEffect;

#[derive(Debug, Clone)]
pub enum EffectResponse {
    System(SystemEffect),
    // ...
}
