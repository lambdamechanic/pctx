use schemars::schema::{InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec};

pub fn anything_schema() -> Schema {
    Schema::Object(SchemaObject::default())
}

pub fn map_schema(value_schema: &Schema) -> Schema {
    let obj = SchemaObject {
        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Object))),
        object: Some(Box::new(ObjectValidation {
            additional_properties: Some(Box::new(value_schema.clone())),
            ..Default::default()
        })),
        ..Default::default()
    };

    Schema::Object(obj)
}
