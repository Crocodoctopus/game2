use std::collections::HashMap;

struct ColliderHandle(u32);

trait ColliderGroup {
    fn apply(&self, other: &Self) -> bool;
}

enum Collider {
    Circle { x: f32, y: f32, r: f32 },
}

impl Collider {
    fn detect(c0: &Self, c1: &Self) -> bool {
        match (c0, c1) {
            // Circle <=> Circle
            (
                Collider::Circle {
                    x: x0,
                    y: y0,
                    r: r0,
                },
                Collider::Circle {
                    x: x1,
                    y: y1,
                    r: r1,
                },
            ) => {
                let dx = x1 - x0;
                let dy = y1 - y0;
                let sr = r0 + r1;
                dx * dx + dy * dy == sr * sr
            }
        }
    }
}

struct CollisionGroup<T, G> {
    counter: u32,
    index_map: HashMap<ColliderHandle, u16>,

    // Data.
    handles: Vec<ColliderHandle>,
    targets: Vec<G>,
    colliders: Vec<Collider>,
    user_data: Vec<T>,
}

impl<T, G> CollisionGroup<T, G> {
    fn new() -> Self {
        Self {
            counter: 0,
            index_map: HashMap::new(),
            handles: vec![],
            targets: vec![],
            colliders: vec![],
            user_data: vec![],
        }
    }

    fn insert(&mut self, target: G, t: T, collider: Collider) -> ColliderHandle {
        let handle = ColliderHandle(self.counter);
        self.counter += 1;
        let index = self.handles.len();

        self.index_map.insert(handle, index);
        self.handles.push(handle);
        self.targets.push(target);
        self.colliders.push(collider);
        self.user_data(t);

        handle
    }

    fn remove(&mut self, handle: ColliderHandle) {
        let Some(index) = self.index_map.remove(handle) else {
            // Handle not in container.
            return;
        };

        self.handles.swap_remove(index as usize);
        self.targets.swap_remove(index as usize);
        self.colliders.swap_remove(index as usize);
        self.user_data.swap_remove(index as usize);

        if index < self.handles.len() {
            self.index_map[self.handles[index as usize] as usize] = index;
        }
    }
}

struct CollisionSystem<T, G> {
    groups: HashMap<G, CollisionGroup<T, G>>,
}

impl<T, G> CollisionSystem<T, G> {
    fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    fn generate_events<'a>(&'a mut self) -> HashMap<ColliderHandle, Vec<&'a T>> {
        let mut out = HashMap::new();
        for (_, group0) in self.groups.iter() {
            for i in 0..group0.handles.len() {
                // Get data related to "source" collider.
                let target = &group0.target[i];
                for (mask, group1) in self.groups.iter() {
                    if !target.apply(mask) {
                        continue;
                    }

                    for j in 0..group1.handles.len() {
                        let collider0 = &group0.colliders[i];
                        let collider1 = &group1.colliders[j];
                        if Collider::detect(&collider0, &collider1) {
                            let handle0 = group0.handles[i];
                            let user_data1 = &group1.user_data[j];
                            out.entry(handle0).or_default().push(user_data1);
                        }
                    }
                }
            }
        }
        return out;
    }

    fn insert(&mut self, group: G, target: G, t: T, collider: Collider) -> ColliderHandle {
        self.groups
            .entry(group)
            .or_default()
            .insert(target, t, collider)
    }
}

mod test {
    use super::*;

    struct Group(u8);

    impl ColliderGroup for Group {
        fn apply(&self, other: &Self) -> bool {
             
        }
    }

    #[test]
    fn collision_test() {
        let mut col_sys = CollisionSystem::new();
        let team0 = 0;
        let team1 = 1;
        let hit0 = col_sys.register_hitbox(0, team0, Collider::Circle { x: 0., y: 0., r: 2 });
        let hit1 = col_sys.register_hitbox(0, team0, Collider::Circle { x: 5., y: 5., r: 2 });
        let hurt0 = col_sys.register_hurtbox(1, team0, Collider::Circle { x: 3., y: 0., r: 2 });
        let hurt1 = col_sys.register_hurtbox(2, team1, Collider::Circle { x: 3., y: 0., r: 2 });

        col_sys.update();

        let events0 = col_sys.get_events(hit0);
        let events1 = col_sys.get_events(hit1);
        let events2 = col_sys.get_events(hurt0);
        let events3 = col_sysm.get_events(hurt1);

        assert_eq!(events0, &[2]);
        assert_eq!(events1, &[]);
        assert_eq!(events2, &[]);
        assert_eq!(events3, &[0]);
    }
}
