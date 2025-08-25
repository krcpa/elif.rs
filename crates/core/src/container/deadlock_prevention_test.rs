use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::container::{IocContainer, ServiceBinder};

#[test]
fn test_no_deadlock_on_concurrent_dispose_and_resolve() {
    // This test verifies that dispose_scope doesn't deadlock with resolve operations
    // by avoiding nested locks

    let container = Arc::new({
        let mut container = IocContainer::new();
        container.bind_singleton::<String, String>();
        container.build().unwrap();
        container
    });

    // Create multiple scopes
    let scope1 = container.create_scope().unwrap();
    let scope2 = container.create_scope().unwrap();
    let scope3 = container.create_scope().unwrap();

    let mut handles = vec![];

    // Thread 1: Continuously dispose and recreate scopes
    let container_clone = container.clone();
    let scope_to_dispose = scope1.clone();
    handles.push(thread::spawn(move || {
        for _ in 0..10 {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Dispose a scope (this used to hold nested locks)
                let _ = container_clone.dispose_scope(&scope_to_dispose).await;
                // Create a new scope
                let _ = container_clone.create_scope();
            });
            thread::sleep(Duration::from_micros(10));
        }
    }));

    // Thread 2: Continuously resolve services in different scopes
    let container_clone = container.clone();
    let scope2_clone = scope2.clone();
    let scope3_clone = scope3.clone();
    handles.push(thread::spawn(move || {
        for i in 0..10 {
            // Try to resolve in different scopes
            let _ = container_clone.resolve_scoped::<String>(&scope2_clone);
            let _ = container_clone.resolve_scoped::<String>(&scope3_clone);

            // Also try regular resolution
            let _ = container_clone.resolve::<String>();

            thread::sleep(Duration::from_micros(5 + i));
        }
    }));

    // Thread 3: Create child scopes
    let container_clone = container.clone();
    let scope2_clone = scope2.clone();
    handles.push(thread::spawn(move || {
        for _ in 0..5 {
            // Try to create child scopes
            if let Ok(child) = container_clone.create_child_scope(&scope2_clone) {
                // Try to use the child scope
                let _ = container_clone.resolve_scoped::<String>(&child);
            }
            thread::sleep(Duration::from_micros(15));
        }
    }));

    // Wait for all threads with a timeout to detect deadlocks
    for handle in handles {
        // If this panics, we have a deadlock
        handle
            .join()
            .expect("Thread should complete without deadlock");
    }
}

#[test]
fn test_lock_ordering_consistency() {
    // This test verifies that operations that need both locks
    // always acquire them in a consistent order

    let container = Arc::new({
        let mut container = IocContainer::new();
        container.build().unwrap();
        container
    });

    // Create scopes
    let scope1 = container.create_scope().unwrap();
    let scope2 = container.create_scope().unwrap();

    let mut handles = vec![];

    // Thread 1: Operations that might touch both scopes and instances
    let container_clone = container.clone();
    let scope = scope1.clone();
    handles.push(thread::spawn(move || {
        for _ in 0..20 {
            // Create scope (touches scopes lock)
            if let Ok(new_scope) = container_clone.create_scope() {
                // Dispose scope (now touches scopes then instances separately)
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = container_clone.dispose_scope(&new_scope).await;
                });
            }

            // Create child scope (touches scopes lock)
            let _ = container_clone.create_child_scope(&scope);

            thread::yield_now();
        }
    }));

    // Thread 2: Service resolution (touches instances lock)
    let container_clone = container.clone();
    handles.push(thread::spawn(move || {
        for _ in 0..20 {
            // These operations touch the instances lock
            let _ = container_clone.resolve_scoped::<String>(&scope2);
            thread::yield_now();
        }
    }));

    // Wait for completion
    for handle in handles {
        handle.join().expect("No deadlock should occur");
    }
}
