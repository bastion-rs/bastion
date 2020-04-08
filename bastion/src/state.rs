use state::Container;
use core::ops::DerefMut;
use core::ops::Deref;
use lightproc::{proc_state::{EmptyState, EmptyProcState}, prelude::State as LPState};
use crate::context::BastionContext;
use std::marker::PhantomData as marker;


pub struct SharedState(Container);

impl SharedState
{
    #[inline(always)]
    pub fn new(c: Container) -> SharedState
    {
        SharedState(c)
    }

    #[inline(always)]
    pub fn get<S>(&self) -> &S
    where
        S: LPState
    {
        self.0.get::<S>()
    }

    #[inline(always)]
    pub fn set<S>(&self, s: S) -> bool
    where
        S: LPState
    {
        let s = s as dyn LPState;
        self.0.set(s)
    }

    #[inline(always)]
    pub fn from_ctx<S>(ctx: &'static BastionContext) -> Option<Self>
    where
        S: LPState
    {
        let container = Container::new();
        container.set((*ctx.state()).get::<S>());
        Some(SharedState(container))
    }
}

impl Default for SharedState {
    fn default() -> Self {
        let mut c = Container::new();
        c.set(EmptyState);
        Self(c)
    }
}

// impl<'s> Deref for SharedState<'s>
// {
//     type Target = S;

//     #[inline(always)]
//     fn deref(&self) -> &S {
//         self.container.get::<S>()
//     }
// }


// impl<'s, S> DerefMut for SharedState<'s>
// where
//     S: LPState + 'static
// {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.container.get::<S>()
//     }
// }
