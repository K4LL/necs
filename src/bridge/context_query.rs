use std::marker::PhantomData;

use crate::ecs::{
    query::{ComponentSet, Query, QueryStorageState, Read, ReadAccessMode, Write, WriteAccessMode},
    system::SystemTuple,
    World,
};

// ================================= COMMON =================================
trait AbstractQuery<C, T>
where
    C: ComponentSet,
    T: QueryStorageState,
{
    type Output;
    type Components: ComponentSet;
    type AccessMode: QueryStorageState;
}
impl<C, T> AbstractQuery<C, T> for Query<C, T>
where
    C: ComponentSet,
    T: QueryStorageState,
{
    type Output = Query<C, T>;
    type Components = C;
    type AccessMode = T;
}

trait AbstractReadQuery<C: ComponentSet> {
    type Output;
    type Components: ComponentSet;
    type AccessMode: ReadAccessMode;

    fn query<W: SystemTuple>(world: &mut World<W>) -> Self::Output;
}
impl<C> AbstractReadQuery<C> for Query<C, Read>
where
    C: ComponentSet,
{
    type Output = Query<C, Read>;
    type Components = C;
    type AccessMode = Read;

    fn query<W: SystemTuple>(world: &mut World<W>) -> Self::Output {
        unsafe { world.query_no_sync() }
    }
}

trait AbstractWriteQuery<C: ComponentSet> {
    type Output;
    type Components: ComponentSet;
    type AccessMode: WriteAccessMode;

    fn query<W: SystemTuple>(world: &mut World<W>) -> Self::Output;
}
impl<C> AbstractWriteQuery<C> for Query<C, Write>
where
    C: ComponentSet,
{
    type Output = Query<C, Write>;
    type Components = C;
    type AccessMode = Write;

    fn query<W: SystemTuple>(world: &mut World<W>) -> Self::Output {
        unsafe { world.query_mut_no_sync() }
    }
}

trait QueryPipeline {
    type Output;
    type Components;
    type AccessMode;

    fn query_all<W: SystemTuple>(world: &mut World<W>) -> Self::Output;
}
impl<C: ComponentSet> QueryPipeline for Query<C, Read> {
    type Output = Query<C, Read>;
    type Components = C;
    type AccessMode = Read;

    fn query_all<W: SystemTuple>(world: &mut World<W>) -> Self::Output {
        unsafe { world.query_no_sync() }
    }
}
impl<C: ComponentSet> QueryPipeline for Query<C, Write> {
    type Output = Query<C, Write>;
    type Components = C;
    type AccessMode = Write;

    fn query_all<W: SystemTuple>(world: &mut World<W>) -> Self::Output {
        unsafe { world.query_mut_no_sync() }
    }
}

macro_rules! impl_query_runner {
    ($($name:ident),+) => {
        impl<$($name),+> QueryPipeline for ($($name,)+)
        where
            $($name: QueryPipeline,)+
        {
            type Output = ($($name::Output,)+);
            type Components = ($($name::Components,)+);
            type AccessMode = ($($name::AccessMode,)+);

            fn query_all<W: SystemTuple>(world: &mut World<W>) -> Self::Output {
                ($($name::query_all(world),)+)
            }
        }
    };
}

macro_rules! impl_query_runner_all {
    ($first:ident) => {
        impl_query_runner!($first);
    };
    ($first:ident, $($rest:ident),+) => {
        impl_query_runner!($first, $($rest),+);
        impl_query_runner_all!($($rest),+);
    };
}
impl_query_runner_all!(A, B, C, D, E, F, G);

// ================================= REST =================================

/// This should not be called anywhere besides in the scheduler code and/or ecs code
/// This makes the bridge between queries and the scheduler
/// It is technically not safe so must be used with caution and revised multiple times or where it is used
pub(crate) struct WorldContext<WorldArgs, R, W>
where
    WorldArgs: SystemTuple,
    R: ComponentSet,
    W: ComponentSet,
{
    _marker: PhantomData<(R, W)>,

    world: *const World<WorldArgs>,
}
impl<WorldArgs, R, W> WorldContext<WorldArgs, R, W>
where
    WorldArgs: SystemTuple,
    R: ComponentSet,
    W: ComponentSet,
{
    pub unsafe fn query_all(&self) -> (Query<R, Read>, Query<W, Write>) {
        let world = self.world as *mut World<WorldArgs>;
        unsafe {
            (
                (*world).query_no_sync::<R>(),
                (*world).query_mut_no_sync::<W>(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Pos(f32);
    struct Vel(f32);
    crate::register_components!(Pos, Vel);

    fn build_world(count: usize) -> World<()> {
        let mut world = World::new().init();
        for i in 0..count {
            let e = world.spawn();
            world.add_component(e, Pos(i as f32));
            world.add_component(e, Vel(i as f32 * 10.0));
        }
        world
    }

    fn context(world: &World<()>) -> WorldContext<(), (Pos,), (Vel,)> {
        WorldContext {
            _marker: PhantomData,
            world: world as *const _,
        }
    }

    #[test]
    fn query_all_returns_read_and_write_views() {
        let world = build_world(3);
        let ctx = context(&world);

        let (mut read, mut write) = unsafe { ctx.query_all() };

        let pos_sum: f32 = read.iter().map(|p| p.0).sum();
        let vel_sum: f32 = write.iter().map(|v| v.0).sum();

        assert_eq!(pos_sum, 0.0 + 1.0 + 2.0);
        assert_eq!(vel_sum, 0.0 + 10.0 + 20.0);
    }

    #[test]
    fn query_all_counts_every_entity() {
        let world = build_world(5);
        let ctx = context(&world);

        let (mut read, mut write) = unsafe { ctx.query_all() };

        assert_eq!(read.iter().count(), 5);
        assert_eq!(write.iter().count(), 5);
    }

    #[test]
    fn query_all_on_empty_world_is_empty() {
        let world = build_world(0);
        let ctx = context(&world);

        let (mut read, mut write) = unsafe { ctx.query_all() };

        assert_eq!(read.iter().count(), 0);
        assert_eq!(write.iter().count(), 0);
    }

    #[test]
    fn query_all_skips_entities_missing_components() {
        let mut world = build_world(2);
        let e = world.spawn();
        world.add_component(e, Pos(100.0));

        let ctx = context(&world);
        let (mut read, mut write) = unsafe { ctx.query_all() };

        assert_eq!(read.iter().count(), 3);
        assert_eq!(write.iter().count(), 2);
    }

    #[test]
    fn pipeline_tuple_queries_everything() {
        let mut world = build_world(4);

        type Args = (Query<(Pos,), Read>, Query<(Vel,), Write>);
        let (mut read, mut write) = <Args as QueryPipeline>::query_all(&mut world);

        assert_eq!(read.iter().count(), 4);
        assert_eq!(write.iter().count(), 4);
    }

    #[test]
    fn pipeline_single_query() {
        let mut world = build_world(2);

        let mut read = <Query<(Pos, Vel), Read> as QueryPipeline>::query_all(&mut world);
        let total: f32 = read.iter().map(|(p, v)| p.0 + v.0).sum();

        assert_eq!(total, (0.0 + 0.0) + (1.0 + 10.0));
    }

    #[test]
    fn write_query_mutates_components() {
        let world = build_world(3);
        let ctx = context(&world);

        let (_, mut write) = unsafe { ctx.query_all() };
        for v in write.iter() {
            v.0 += 1.0;
        }

        let (_, mut write) = unsafe { ctx.query_all() };
        let sum: f32 = write.iter().map(|v| v.0).sum();
        assert_eq!(sum, 33.0);
    }
}
