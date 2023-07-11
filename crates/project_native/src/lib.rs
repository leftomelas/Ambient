use std::collections::HashMap;

use ambient_ecs::{ExternalComponentAttributes, ExternalComponentDesc, PrimitiveComponentType};
use ambient_project_semantic::{Item, TypeInner};

pub fn all_defined_components(
    semantic: &ambient_project_semantic::Semantic,
) -> anyhow::Result<Vec<ExternalComponentDesc>> {
    let items = &semantic.items;
    let root_scope = &semantic.root_scope();

    let type_map = {
        let mut type_map = HashMap::new();

        // First pass: add all root-level primitive types
        for type_id in root_scope.types.values() {
            let type_ = items.get(*type_id).expect("type id not in items");
            if let TypeInner::Primitive(pt) = type_.inner {
                let ty = PrimitiveComponentType::try_from(pt.to_string().as_str()).unwrap();
                type_map.insert(*type_id, ty);
                type_map.insert(items.get_vec_id(*type_id), ty.to_vec_type().unwrap());
                type_map.insert(items.get_option_id(*type_id), ty.to_option_type().unwrap());
            }
        }

        // Second pass: traverse the type graph and add all enums
        root_scope.visit_recursive(items, |scope| {
            for type_id in scope.types.values() {
                let type_ = items.get(*type_id).expect("type id not in items");
                if let TypeInner::Enum { .. } = type_.inner {
                    type_map.insert(*type_id, PrimitiveComponentType::U32);
                }
            }
            Ok(())
        })?;

        type_map
    };

    let mut components = vec![];
    root_scope.visit_recursive(&items, |scope| {
        for id in scope.components.values().copied() {
            let component = items.get(id)?;

            let attributes: Vec<_> = component
                .attributes
                .iter()
                .map(|id| {
                    let attr = items.get(id.as_resolved().unwrap())?;
                    Ok(attr.data().id.as_upper_camel_case())
                })
                .collect::<anyhow::Result<_>>()?;

            components.push(ExternalComponentDesc {
                path: items.fully_qualified_display_path_ambient_style(&*component)?,
                ty: type_map[&component.type_.as_resolved().unwrap()],
                name: component.name.clone(),
                description: component.description.clone(),
                attributes: ExternalComponentAttributes::from_iter(
                    attributes.iter().map(|s| s.as_str()),
                ),
            });
        }
        Ok(())
    })?;
    Ok(components)
}
