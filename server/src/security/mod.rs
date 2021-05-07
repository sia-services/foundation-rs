mod identity;
mod authorization;

use std::collections::HashSet;

pub struct SecurityContext {
    user_id: u32,    // this is ID of user
    groups:  HashSet<String>,
}

impl SecurityContext {
    pub fn new(user_id: u32, groups:  HashSet<String>) -> Self {
        Self { user_id, groups }
    }
}

pub use identity::IdentityService;
pub use authorization::Authorized;
