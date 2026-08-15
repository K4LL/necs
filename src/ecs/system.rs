use std::marker::PhantomData;

use crate::ecs::World;
use crate::ecs::query::{ComponentSet, Query};

// Trait for systems
pub trait SystemFunction<F, C>
where
    F: Fn(Query<C>),
    C: ComponentSet,
{
    type Components: ComponentSet;

    fn run<T: SystemTuple>(&self, world: &mut World<T>);
}
impl<F, C> SystemFunction<F, C> for F
where
    F: Fn(Query<C>),
    C: ComponentSet,
{
    type Components = C;

    fn run<T: SystemTuple>(&self, world: &mut World<T>) {
        let query = world.query::<C>();
        (self)(query);
    }
}

// Trait for systems but with no query or restraints
// Note: W is required by the world, but has nothing to do with this trait
pub trait AbstractSystem {
    fn run_system<W: SystemTuple>(&self, world: &mut World<W>);
}
impl AbstractSystem for () {
    #[inline]
    fn run_system<W>(&self, _world: &mut World<W>) {
        // Empty base case
    }
}

// Struct that allows FunctionSystems to be turned into AbstractSystems
pub struct SystemWrapper<F, C> {
    pub system: F,
    pub _marker: PhantomData<C>,
}
impl<F, C> AbstractSystem for SystemWrapper<F, C>
where
    F: SystemFunction<F, C> + Fn(Query<C>),
    C: ComponentSet,
{
    #[inline]
    fn run_system<W: SystemTuple>(&self, world: &mut World<W>) {
        self.system.run(world);
    }
}

// Trait to flatten tuples
pub trait Flatten {
    type Output;
    fn flatten(self) -> Self::Output;
}
// Impl for empty tuples
impl Flatten for () {
    type Output = ();

    #[inline]
    fn flatten(self) -> Self::Output {
        ()
    }
}
// Base case for AbstractSystem tuples
impl<A: AbstractSystem> Flatten for ((), A) {
    type Output = (A,);

    #[inline]
    fn flatten(self) -> Self::Output {
        (self.1,)
    }
}

// Macro to help implement Flatten for multiple tuple sizes
macro_rules! impl_flatten {
    // Entry point for recursive matching
    ($head:ident $(, $tail:ident)* $(,)?) => {
        impl_flatten!(@step ($head) ; ($($tail),*));
    };

    // Base case for when no identifiers are left
    (@step ($($current:ident),*) ; ()) => {
        impl_flatten!(@impl ($($current),*));
    };

    // Implement for the current tuple, then recurse with the next
    (@step ($($current:ident),*) ; ($next_head:ident $(, $next_tail:ident)*)) => {
        impl_flatten!(@impl ($($current),*));
        impl_flatten!(@step ($($current,)* $next_head) ; ($($next_tail),*));
    };

    // Generate impl for a single tuple size
    (@impl ($($T:ident),*)) => {
        impl<$($T),*, NEXT> Flatten for (($($T,)*), NEXT) {
            type Output = ($($T,)* NEXT,);

            #[allow(non_snake_case)]
            #[inline]
            fn flatten(self) -> Self::Output {
                let (($($T,)*), next) = self;
                ($($T,)* next,)
            }
        }
    };
}
impl_flatten!(A, B, C, D, E, F, G, H, I, J);

// Trait to allow visiting single systems
pub trait SystemVisitor {
    fn visit<W: SystemTuple, S: AbstractSystem>(&mut self, system: &mut S, world: &mut World<W>);
}

// Trait for a tuple of systems
pub trait SystemTuple {
    fn accept_visitor<W: SystemTuple, V: SystemVisitor>(
        &mut self,
        visitor: &mut V,
        world: &mut World<W>,
    );
}
// Impl for empty tuples
impl SystemTuple for () {
    #[inline]
    fn accept_visitor<W, V: SystemVisitor>(&mut self, _visitor: &mut V, _world: &mut World<W>) {
        // Empty base case
    }
}

// Helper to implement the system tuple for each tuple size
macro_rules! impl_system_tuple {
    ($( ($($idx:tt : $T:ident),*) );* $(;)?) => {
        $(
            impl<$($T: AbstractSystem),*> SystemTuple for ($($T,)*) {
                #[inline]
                fn accept_visitor<W: SystemTuple, V: SystemVisitor>(&mut self, visitor: &mut V, world: &mut World<W>) {
                    $( visitor.visit(&mut self.$idx, world); )*
                }
            }
        )*
    };
}
impl_system_tuple! {
    (0: A);
    (0: A, 1: B);
    (0: A, 1: B, 2: C);
    (0: A, 1: B, 2: C, 3: D);
    (0: A, 1: B, 2: C, 3: D, 4: E);
    (0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
    (0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
    (0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
    (0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I);
    (0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J);
}

// A simple runner that allows systems to be visited
pub struct SystemRunner;
impl SystemVisitor for SystemRunner {
    #[inline]
    fn visit<W: SystemTuple, S: AbstractSystem>(&mut self, system: &mut S, world: &mut World<W>) {
        system.run_system(world);
    }
}
