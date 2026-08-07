use::necs::{ World, Entity };

#[test]
fn full_test() {
    let mut world = World::new();

    let entity = world.spawn();
    let second_entity = world.spawn();

    let val = 32;
    world.add_component::<i32>(entity, val);

    let sval = 101243;
    world.add_component::<i32>(second_entity, sval);

    let third_entity = world.spawn();
    let tval = 49.5;
    world.add_component(third_entity, tval);

    let component = world.get_component::<i32>(entity);
    assert!(val == *component);
    assert!(sval == *world.get_component::<i32>(second_entity));
    assert!(tval == *world.get_component::<f32>(third_entity));

    world.despawn(entity);
    world.despawn(second_entity);
}
