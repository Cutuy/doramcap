use prost::Message;
use prost_types::{DescriptorProto, EnumDescriptorProto, FileDescriptorSet};

fn enums(old: &[EnumDescriptorProto], new: &[EnumDescriptorProto]) -> Result<(), String> {
    for a in old {
        let b = new
            .iter()
            .find(|b| b.name == a.name)
            .ok_or("removed enum")?;
        for v in &b.value {
            if a.reserved_name
                .contains(&v.name.clone().unwrap_or_default())
                || a.reserved_range
                    .iter()
                    .any(|r| r.start <= v.number && v.number <= r.end)
            {
                return Err("reused enum reservation".into());
            }
        }
        for v in &a.value {
            if let Some(w) = b.value.iter().find(|w| w.name == v.name) {
                if w.number != v.number {
                    return Err("renumbered enum".into());
                }
            } else if !b
                .reserved_name
                .contains(&v.name.clone().unwrap_or_default())
                || !b
                    .reserved_range
                    .iter()
                    .any(|r| r.start <= v.number && v.number <= r.end)
            {
                return Err("unreserved enum deletion".into());
            }
            if b.value.iter().any(|w| {
                w.number == v.number
                    && !a
                        .value
                        .iter()
                        .any(|old| old.name == w.name && old.number == w.number)
            }) {
                return Err("reused enum number".into());
            }
        }
        for name in &a.reserved_name {
            if !b.reserved_name.contains(name) {
                return Err("removed enum reservation".into());
            }
        }
        for r in &a.reserved_range {
            if !b
                .reserved_range
                .iter()
                .any(|s| s.start <= r.start && s.end >= r.end)
            {
                return Err("removed enum reservation".into());
            }
        }
    }
    Ok(())
}
fn messages(old: &[DescriptorProto], new: &[DescriptorProto]) -> Result<(), String> {
    for a in old {
        let b = new
            .iter()
            .find(|b| b.name == a.name)
            .ok_or("removed message")?;
        for f in &b.field {
            if a.reserved_name
                .contains(&f.name.clone().unwrap_or_default())
                || a.reserved_range
                    .iter()
                    .any(|r| r.start <= f.number && f.number < r.end)
            {
                return Err("reused field reservation".into());
            }
        }
        for f in &a.field {
            if let Some(g) = b.field.iter().find(|g| g.number == f.number) {
                let oneof = |m: &DescriptorProto, i: Option<i32>| {
                    i.and_then(|i| m.oneof_decl.get(i as usize))
                        .and_then(|o| o.name.clone())
                };
                if f.name != g.name
                    || f.r#type != g.r#type
                    || f.type_name != g.type_name
                    || f.label != g.label
                    || f.proto3_optional != g.proto3_optional
                    || f.default_value != g.default_value
                    || oneof(a, f.oneof_index) != oneof(b, g.oneof_index)
                {
                    return Err("incompatible field".into());
                }
            } else if !b
                .reserved_name
                .contains(&f.name.clone().unwrap_or_default())
                || !b
                    .reserved_range
                    .iter()
                    .any(|r| r.start <= f.number && f.number < r.end)
            {
                return Err("unreserved field deletion".into());
            }
            if b.field
                .iter()
                .any(|g| g.name == f.name && g.number != f.number)
            {
                return Err("renumbered field".into());
            }
        }
        for name in &a.reserved_name {
            if !b.reserved_name.contains(name) {
                return Err("removed field reservation".into());
            }
        }
        for r in &a.reserved_range {
            if !b
                .reserved_range
                .iter()
                .any(|s| s.start <= r.start && s.end >= r.end)
            {
                return Err("removed field reservation".into());
            }
        }
        messages(&a.nested_type, &b.nested_type)?;
        enums(&a.enum_type, &b.enum_type)?;
    }
    Ok(())
}
fn compatible(old: &FileDescriptorSet, new: &FileDescriptorSet) -> Result<(), String> {
    for a in &old.file {
        let b = new
            .file
            .iter()
            .find(|b| b.name == a.name)
            .ok_or("removed file")?;
        if a.package != b.package || a.syntax != b.syntax {
            return Err("changed package/syntax".into());
        }
        messages(&a.message_type, &b.message_type)?;
        enums(&a.enum_type, &b.enum_type)?;
    }
    Ok(())
}
fn baseline() -> FileDescriptorSet {
    FileDescriptorSet::decode(include_bytes!("baselines/v1.bin").as_slice()).unwrap()
}
#[test]
fn current_schema_preserves_v1() {
    compatible(
        &baseline(),
        &FileDescriptorSet::decode(fault_schema::FILE_DESCRIPTOR_SET).unwrap(),
    )
    .unwrap();
}
#[test]
fn breaking_field_changes_fail() {
    let old = baseline();
    for mutation in 0..7 {
        let mut new = old.clone();
        let f = &mut new.file[0].message_type[0].field[0];
        match mutation {
            0 => f.number = Some(99),
            1 => f.name = Some("reused".into()),
            2 => f.r#type = Some(1),
            3 => f.label = Some(3),
            4 => f.proto3_optional = Some(true),
            5 => f.type_name = Some(".different.Message".into()),
            _ => {
                new.file[0].message_type[0].field.remove(0);
            }
        }
        assert!(compatible(&old, &new).is_err(), "mutation {mutation}");
    }
    let mut new = old.clone();
    let m = new
        .file
        .iter_mut()
        .flat_map(|f| &mut f.message_type)
        .find(|m| !m.oneof_decl.is_empty())
        .unwrap();
    m.oneof_decl[0].name = Some("changed_oneof".into());
    assert!(compatible(&old, &new).is_err());
}
#[test]
fn enum_renumbering_and_deletion_fail() {
    let old = baseline();
    let mut new = old.clone();
    new.file[0].enum_type[0].value[1].number = Some(99);
    assert!(compatible(&old, &new).is_err());
    new = old.clone();
    new.file[0].enum_type[0].value.remove(1);
    assert!(compatible(&old, &new).is_err());
}
#[test]
fn reserved_deletions_and_additions_pass_but_reuse_fails() {
    let old = baseline();
    let mut new = old.clone();
    let m = &mut new.file[0].message_type[0];
    let removed = m.field.remove(0);
    m.reserved_name.push(removed.name.unwrap());
    m.reserved_range
        .push(prost_types::descriptor_proto::ReservedRange {
            start: removed.number,
            end: removed.number.map(|n| n + 1),
        });
    compatible(&old, &new).unwrap();
    let mut future = new.clone();
    future.file[0].message_type[0]
        .field
        .push(old.file[0].message_type[0].field[0].clone());
    assert!(compatible(&new, &future).is_err());
    let mut added = old.clone();
    let m = &mut added.file[0].message_type[0];
    let mut f = m.field[0].clone();
    f.name = Some("new_field".into());
    f.number = Some(100);
    m.field.push(f);
    compatible(&old, &added).unwrap();
}
