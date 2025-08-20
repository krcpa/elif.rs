# Blog with Observers Example

This example demonstrates the model events and observers system in elif.rs, showing how to implement lifecycle hooks for database models.

## Features Demonstrated

- **Model Events**: Creating, created, updating, updated, saving, saved, deleting, deleted
- **Observer Pattern**: Multiple observers per model with clean separation of concerns  
- **Email Normalization**: Automatic email lowercasing during user creation
- **Slug Generation**: Automatic URL-friendly slug generation for blog posts
- **Timestamp Management**: Automatic published_at timestamp handling
- **Audit Logging**: Comprehensive audit trail for all model changes
- **Security Monitoring**: Special security audit observer for sensitive operations
- **Validation**: Model validation with proper error handling
- **Error Propagation**: How errors in observers stop the event chain

## Project Structure

```
blog-with-observers/
├── main.rs                     # Example application
├── Cargo.toml                  # Dependencies
├── README.md                   # This file
└── observers/
    ├── user_observer.rs        # User model observer
    ├── post_observer.rs        # Post model observer  
    └── audit_observer.rs       # Generic audit observer
```

## Running the Example

```bash
cd examples/blog-with-observers
cargo run
```

## Observer Implementations

### UserObserver

Handles user-specific logic:

- **Creating**: Normalizes email to lowercase, validates uniqueness
- **Created**: Sends welcome email, creates user profile
- **Updating**: Logs field changes, normalizes email updates
- **Updated**: Sends email change notifications
- **Deleting**: Logs deletion attempts
- **Deleted**: Cleans up related data

### PostObserver

Handles blog post logic:

- **Creating**: Generates URL-friendly slug, validates title, sets published timestamp
- **Created**: Indexes for search, warms cache
- **Updating**: Updates slug on title change, manages published timestamps
- **Updated**: Handles search index updates, invalidates/warms cache
- **Deleting**: Checks dependencies
- **Deleted**: Removes from search index, cleans cache

### AuditObserver

Generic audit logging:

- **Created**: Logs INSERT operations with new values
- **Updated**: Logs UPDATE operations with old and new values  
- **Deleted**: Logs DELETE operations with final values
- Includes user context (user ID, IP address, user agent)
- Timestamps all operations

### SecurityAuditObserver

Security-focused monitoring:

- **Creating**: Logs user creation with security context
- **Updating**: Monitors sensitive field changes (email changes)
- **Deleting**: Logs account deletions with security implications
- Could integrate with security systems for alerts

## Key Concepts

### Event Flow

For a typical **create** operation:
1. `creating` event - validation and preparation
2. `saving` event - just before database save
3. `saved` event - just after database save  
4. `created` event - final post-creation tasks

For a typical **update** operation:
1. `updating` event - validation and preparation
2. `saving` event - just before database save
3. `saved` event - just after database save
4. `updated` event - final post-update tasks

### Error Handling

Observers can return errors that will:

- Stop the event propagation chain
- Prevent the database operation (for `creating`, `updating` events)
- Be returned to the calling code for handling

### Multiple Observers

Multiple observers can be registered for the same model:

- They execute in registration order
- If any observer fails, the chain stops
- Each observer can modify the model (for mutable events)
- Observers are independent and focused on single concerns

## Real-World Usage

In a production application, you might:

- Use observers for audit logging (compliance requirements)
- Implement search indexing (Elasticsearch, Algolia)  
- Handle cache invalidation (Redis, Memcached)
- Send notifications (email, push, webhooks)
- Validate business rules
- Update related models
- Generate derived data (slugs, thumbnails, etc.)
- Monitor security events
- Implement soft deletes
- Handle file uploads/deletions

## Performance Considerations

- Observers run synchronously in the request cycle
- Long-running operations should use background jobs
- Consider caching for expensive operations
- Database operations in observers should be efficient
- Error handling is critical for user experience

## Testing

The example includes comprehensive tests showing:

- Individual observer functionality
- Integration between multiple observers
- Error handling and validation
- Full model lifecycle flows

Run the tests with:

```bash
cargo test
```

This demonstrates how the event system enables clean separation of concerns while maintaining consistency and reliability in your data layer.