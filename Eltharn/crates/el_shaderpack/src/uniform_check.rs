use naga::{Handle, Module, Type};
fn is_v4u(m: &Module, h: Handle<Type>) -> bool {
    matches!(
        &m.types[h].inner,
        naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            scalar: naga::Scalar {
                kind: naga::ScalarKind::Uint,
                width: 4
            }
        }
    )
}
fn is_v4f(m: &Module, h: Handle<Type>) -> bool {
    matches!(
        &m.types[h].inner,
        naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            scalar: naga::Scalar {
                kind: naga::ScalarKind::Float,
                width: 4
            }
        }
    )
}
fn is_m44f(m: &Module, h: Handle<Type>) -> bool {
    matches!(
        &m.types[h].inner,
        naga::TypeInner::Matrix {
            columns: naga::VectorSize::Quad,
            rows: naga::VectorSize::Quad,
            scalar: naga::Scalar {
                kind: naga::ScalarKind::Float,
                width: 4
            }
        }
    )
}
pub(crate) fn params(m: &naga::Module, ty: naga::Handle<naga::Type>) -> bool {
    let (span, members) = match &m.types[ty].inner {
        naga::TypeInner::Struct { members, span } => (*span, members),
        _ => return false,
    };
    if span != 1024 || members.len() != 1 {
        return false;
    }
    let mb = &members[0];
    if mb.name.as_deref() != Some("values") || mb.offset != 0 || mb.binding.is_some() {
        return false;
    }
    match &m.types[mb.ty].inner {
        naga::TypeInner::Array { base, size, stride } => {
            if *stride != 16 {
                return false;
            }
            if !matches!(size, naga::ArraySize::Constant(n) if n.get() == 64) {
                return false;
            }
            is_v4u(m, *base)
        }
        _ => false,
    }
}
pub(crate) fn camera(m: &naga::Module, ty: naga::Handle<naga::Type>) -> bool {
    let (span, members) = match &m.types[ty].inner {
        naga::TypeInner::Struct { members, span } => (*span, members),
        _ => return false,
    };
    if span != 176 || members.len() != 5 {
        return false;
    }
    let exp: [(&str, u32, bool); 5] = [
        ("view_projection", 0, true),
        ("inverse_view_projection", 64, true),
        ("camera", 128, false),
        ("viewport", 144, false),
        ("planes", 160, false),
    ];
    for (i, (n, off, is_mat)) in exp.iter().enumerate() {
        let mb = &members[i];
        if mb.name.as_deref() != Some(*n) || mb.offset != *off || mb.binding.is_some() {
            return false;
        }
        if *is_mat {
            if !is_m44f(m, mb.ty) {
                return false;
            }
        } else if !is_v4f(m, mb.ty) {
            return false;
        }
    }
    true
}
pub(crate) fn environment(m: &naga::Module, ty: naga::Handle<naga::Type>) -> bool {
    let (span, members) = match &m.types[ty].inner {
        naga::TypeInner::Struct { members, span } => (*span, members),
        _ => return false,
    };
    if span != 80 || members.len() != 5 {
        return false;
    }
    let exp: [(&str, u32, bool); 5] = [
        ("time_words", 0, true),
        ("weather", 16, false),
        ("celestial", 32, false),
        ("sky_light_color", 48, false),
        ("moon_phase", 64, true),
    ];
    for (i, (n, off, is_uint)) in exp.iter().enumerate() {
        let mb = &members[i];
        if mb.name.as_deref() != Some(*n) || mb.offset != *off || mb.binding.is_some() {
            return false;
        }
        if *is_uint {
            if !is_v4u(m, mb.ty) {
                return false;
            }
        } else if !is_v4f(m, mb.ty) {
            return false;
        }
    }
    true
}
pub(crate) fn history(m: &Module, ty: Handle<Type>) -> bool {
    let naga::TypeInner::Struct { members, span } = &m.types[ty].inner else {
        return false;
    };
    *span == 192
        && members.len() == 2
        && members[0].name.as_deref() == Some("previous_camera")
        && members[0].offset == 0
        && members[0].binding.is_none()
        && camera(m, members[0].ty)
        && members[1].name.as_deref() == Some("status")
        && members[1].offset == 176
        && members[1].binding.is_none()
        && is_v4u(m, members[1].ty)
}
pub(crate) fn shadow(m: &Module, ty: Handle<Type>) -> bool {
    let naga::TypeInner::Struct { members, span } = &m.types[ty].inner else {
        return false;
    };
    *span == 96
        && members.len() == 3
        && members
            .iter()
            .zip([
                ("view_projection", 0, true),
                ("light_direction", 64, false),
                ("map", 80, false),
            ])
            .all(|(member, (name, offset, matrix))| {
                member.name.as_deref() == Some(name)
                    && member.offset == offset
                    && member.binding.is_none()
                    && if matrix {
                        is_m44f(m, member.ty)
                    } else {
                        is_v4f(m, member.ty)
                    }
            })
}
