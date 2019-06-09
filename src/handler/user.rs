use metrohash::MetroHashMap;
use std::collections::HashSet;
use actix::prelude::*;

/// Service for handling user related things
pub struct UserService {
    active_users: MetroHashMap<usize,User>,
}

impl Default for UserService {
    fn default() -> Self {
        Self {
            active_users: MetroHashMap::default(),
        }
    }
}

/// Active user in system
pub struct User {
    pub id: usize,
    pub name: String,
    pub password: String,
    pub permissions: HashSet<String>,
}

impl SystemService for UserService {}
impl Supervised for UserService {}
impl Actor for UserService {
    type Context = Context<Self>;
}