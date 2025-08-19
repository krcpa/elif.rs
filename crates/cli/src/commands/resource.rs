use elif_core::{ElifError, ResourceSpec, StorageSpec, specs::FieldSpec, ApiSpec, OperationSpec};
use std::path::PathBuf;

pub fn new_resource(name: &str, route: &str, fields_str: &str) -> Result<(), ElifError> {
    let fields = parse_fields(fields_str)?;
    
    let spec = ResourceSpec {
        kind: "Resource".to_string(),
        name: name.to_string(),
        route: route.to_string(),
        storage: StorageSpec {
            table: pluralize(name).to_lowercase(),
            soft_delete: false,
            timestamps: true,
            fields,
        },
        indexes: vec![],
        uniques: vec![],
        relations: vec![],
        api: ApiSpec {
            operations: vec![
                OperationSpec {
                    op: "create".to_string(),
                    method: "POST".to_string(),
                    path: "/".to_string(),
                    paging: None,
                    filter: None,
                    search_by: None,
                    order_by: None,
                },
                OperationSpec {
                    op: "list".to_string(),
                    method: "GET".to_string(),
                    path: "/".to_string(),
                    paging: Some("cursor".to_string()),
                    filter: None,
                    search_by: None,
                    order_by: Some(vec!["created_at".to_string()]),
                },
                OperationSpec {
                    op: "get".to_string(),
                    method: "GET".to_string(),
                    path: "/:id".to_string(),
                    paging: None,
                    filter: None,
                    search_by: None,
                    order_by: None,
                },
                OperationSpec {
                    op: "update".to_string(),
                    method: "PATCH".to_string(),
                    path: "/:id".to_string(),
                    paging: None,
                    filter: None,
                    search_by: None,
                    order_by: None,
                },
                OperationSpec {
                    op: "delete".to_string(),
                    method: "DELETE".to_string(),
                    path: "/:id".to_string(),
                    paging: None,
                    filter: None,
                    search_by: None,
                    order_by: None,
                },
            ],
        },
        policy: Default::default(),
        validate: Default::default(),
        examples: Default::default(),
        events: Default::default(),
    };
    
    let resources_dir = PathBuf::from("resources");
    std::fs::create_dir_all(&resources_dir).map_err(|e| ElifError::Io(e))?;
    
    let file_path = resources_dir.join(format!("{}.resource.yaml", name.to_lowercase()));
    let yaml = spec.to_yaml()?;
    std::fs::write(file_path, yaml).map_err(|e| ElifError::Io(e))?;
    
    println!("✓ Created resource specification for {}", name);
    Ok(())
}

fn parse_fields(fields_str: &str) -> Result<Vec<FieldSpec>, ElifError> {
    let mut fields = vec![
        FieldSpec {
            name: "id".to_string(),
            field_type: "uuid".to_string(),
            pk: true,
            required: true,
            index: false,
            default: Some("gen_random_uuid()".to_string()),
            validate: None,
        }
    ];
    
    for field_def in fields_str.split(',') {
        let parts: Vec<&str> = field_def.trim().split(':').collect();
        if parts.len() != 2 {
            return Err(ElifError::Validation { message: format!("Invalid field definition: {}. Expected name:type format", field_def) });
        }
        
        fields.push(FieldSpec {
            name: parts[0].trim().to_string(),
            field_type: parts[1].trim().to_string(),
            pk: false,
            required: true,
            index: false,
            default: None,
            validate: None,
        });
    }
    
    Ok(fields)
}

fn pluralize(word: &str) -> String {
    if word.ends_with('y') {
        format!("{}ies", &word[..word.len()-1])
    } else if word.ends_with('s') || word.ends_with("sh") || word.ends_with("ch") {
        format!("{}es", word)
    } else {
        format!("{}s", word)
    }
}