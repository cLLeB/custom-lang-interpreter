use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::Value;

/// A shared reference to an environment scope.
/// Using Rc<RefCell<>> ensures mutations propagate through the scope chain,
/// which is essential for correct while-loop variable updates and closures.
pub type EnvRef = Rc<RefCell<Env>>;

#[derive(Debug, Clone)]
pub struct Env {
    vars: HashMap<String, Value>,
    pub parent: Option<EnvRef>,
}

impl Env {
    /// Create a new root (global) environment
    pub fn root() -> EnvRef {
        Rc::new(RefCell::new(Env {
            vars: HashMap::new(),
            parent: None,
        }))
    }

    /// Create a child environment inheriting from a parent
    pub fn child(parent: &EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Env {
            vars: HashMap::new(),
            parent: Some(Rc::clone(parent)),
        }))
    }

    /// Define a new variable in the current (innermost) scope
    pub fn define(env: &EnvRef, name: &str, value: Value) {
        env.borrow_mut().vars.insert(name.to_string(), value);
    }

    /// Look up a variable, walking up the scope chain
    pub fn get(env: &EnvRef, name: &str) -> Option<Value> {
        let inner = env.borrow();
        if let Some(v) = inner.vars.get(name) {
            return Some(v.clone());
        }
        if let Some(parent) = inner.parent.clone() {
            drop(inner);
            return Env::get(&parent, name);
        }
        None
    }

    /// Assign to an existing variable anywhere in the scope chain.
    /// Returns true if the variable was found and updated.
    pub fn set(env: &EnvRef, name: &str, value: Value) -> bool {
        let mut inner = env.borrow_mut();
        if inner.vars.contains_key(name) {
            inner.vars.insert(name.to_string(), value);
            return true;
        }
        if let Some(parent) = inner.parent.clone() {
            drop(inner);
            return Env::set(&parent, name, value);
        }
        false
    }

    /// Collect all variable names visible from this scope (for error suggestions)
    pub fn all_names(env: &EnvRef) -> Vec<String> {
        let mut names = Vec::new();
        let mut cur = Some(Rc::clone(env));
        while let Some(rc) = cur {
            let inner = rc.borrow();
            names.extend(inner.vars.keys().cloned());
            cur = inner.parent.clone();
        }
        names
    }
}
