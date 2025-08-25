use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::container::IocContainer;
use crate::errors::CoreError;

#[test]
fn test_create_child_scope_atomicity() {
    // This test verifies that create_child_scope is atomic and won't create
    // orphaned children if the parent is disposed concurrently

    let container = Arc::new({
        let mut container = IocContainer::new();
        container.build().unwrap();
        container
    });

    // Create a parent scope
    let parent_scope_id = container.create_scope().unwrap();

    // Spawn multiple threads that will try to:
    // 1. Create child scopes
    // 2. Dispose the parent scope
    let mut handles = vec![];

    // Thread that tries to dispose the parent scope
    let container_clone = container.clone();
    let parent_id = parent_scope_id.clone();
    handles.push(thread::spawn(move || {
        // Small delay to increase chance of race
        thread::sleep(Duration::from_micros(10));

        // Try to dispose parent - this is async so we use tokio
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = container_clone.dispose_scope(&parent_id).await;
        });
    }));

    // Threads that try to create child scopes
    for i in 0..5 {
        let container_clone = container.clone();
        let parent_id = parent_scope_id.clone();
        handles.push(thread::spawn(move || {
            // Varying delays to increase race likelihood
            thread::sleep(Duration::from_micros(i as u64 * 5));

            // Try to create a child scope
            match container_clone.create_child_scope(&parent_id) {
                Ok(child_id) => {
                    // If we successfully created a child, the parent must still exist
                    // Try to create a grandchild to verify the child is valid
                    assert!(
                        container_clone.create_child_scope(&child_id).is_ok(),
                        "Created child scope but it's not valid"
                    );
                }
                Err(CoreError::ServiceNotFound { .. }) => {
                    // Parent was disposed - this is expected
                }
                Err(e) => {
                    panic!("Unexpected error: {:?}", e);
                }
            }
        }));
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_scope_operations() {
    // Test various concurrent operations on scopes
    let container = Arc::new({
        let mut container = IocContainer::new();
        container.build().unwrap();
        container
    });

    // Create some initial scopes
    let scope1 = container.create_scope().unwrap();
    let scope2 = container.create_scope().unwrap();

    let mut handles = vec![];

    // Thread creating children of scope1
    for i in 0..3 {
        let container_clone = container.clone();
        let parent = scope1.clone();
        handles.push(thread::spawn(move || {
            thread::sleep(Duration::from_micros(i * 10));
            let _ = container_clone.create_child_scope(&parent);
        }));
    }

    // Thread creating children of scope2
    for i in 0..3 {
        let container_clone = container.clone();
        let parent = scope2.clone();
        handles.push(thread::spawn(move || {
            thread::sleep(Duration::from_micros(i * 10));
            let _ = container_clone.create_child_scope(&parent);
        }));
    }

    // Thread disposing scope1
    let container_clone = container.clone();
    let scope_to_dispose = scope1.clone();
    handles.push(thread::spawn(move || {
        thread::sleep(Duration::from_micros(25));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = container_clone.dispose_scope(&scope_to_dispose).await;
        });
    }));

    // Wait for all operations to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify scope2 is still valid (scope1 disposal shouldn't affect it)
    assert!(
        container.create_child_scope(&scope2).is_ok(),
        "Scope2 should still be valid after scope1 disposal"
    );
}
