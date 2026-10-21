use crate::model::Profile;
use crate::strict::ensure;
use naga::{
    AddressSpace, Binding, BuiltIn, Handle, ImageClass, ImageDimension, Module, Scalar, ScalarKind,
    ShaderStage, Type, TypeInner, VectorSize,
};
use std::collections::BTreeSet;
type Result<T> = std::result::Result<T, String>;
fn io(
    m: &Module,
    ty: Handle<Type>,
    binding: &Option<Binding>,
) -> Result<Vec<(Binding, Handle<Type>)>> {
    if let Some(b) = binding {
        return Ok(vec![(b.clone(), ty)]);
    }
    if let TypeInner::Struct { members, .. } = &m.types[ty].inner {
        return members
            .iter()
            .map(|x| Ok((x.binding.clone().ok_or("unbound IO member")?, x.ty)))
            .collect();
    }
    Err("unbound IO".into())
}
fn vec_float(m: &Module, ty: Handle<Type>, size: VectorSize) -> bool {
    m.types[ty].inner
        == TypeInner::Vector {
            size,
            scalar: Scalar {
                kind: ScalarKind::Float,
                width: 4,
            },
        }
}
fn uv(b: &Binding) -> bool {
    matches!(
        b,
        Binding::Location {
            location: 0,
            second_blend_source: false,
            interpolation: Some(naga::Interpolation::Perspective),
            sampling: Some(naga::Sampling::Center)
        }
    )
}

pub(crate) fn validate_wgsl(
    source: &str,
    vertex: &str,
    fragment: &str,
    profile: Profile,
) -> Result<bool> {
    validate_wgsl_inner(
        source, vertex, fragment, profile, false, false, false, false,
    )
}

pub(crate) fn validate_frame_wgsl(
    source: &str,
    vertex: &str,
    fragment: &str,
    shadow: bool,
    history: bool,
    light: bool,
) -> Result<bool> {
    validate_wgsl_inner(
        source,
        vertex,
        fragment,
        Profile::ColorDepthEnvironment,
        true,
        shadow,
        history,
        light,
    )
}

fn validate_wgsl_inner(
    source: &str,
    vertex: &str,
    fragment: &str,
    profile: Profile,
    geometry: bool,
    shadow: bool,
    history: bool,
    light: bool,
) -> Result<bool> {
    ensure(
        source.len() <= 1048576 && !source.starts_with('\u{feff}'),
        "WGSL byte bound/BOM",
    )?;
    let m = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&m)
    .map_err(|e| e.to_string())?;
    let raw_depth_only = |function: &naga::Function, fi: &naga::valid::FunctionInfo| {
        function
            .expressions
            .iter()
            .all(|(_, expression)| match *expression {
                naga::Expression::ImageSample { image, .. } => !matches!(
                    fi[image].ty.inner_with(&m.types),
                    TypeInner::Image {
                        class: ImageClass::Depth { .. },
                        ..
                    }
                ),
                _ => true,
            })
    };
    ensure(
        m.functions.iter().all(|(h, f)| raw_depth_only(f, &info[h]))
            && m.entry_points
                .iter()
                .enumerate()
                .all(|(i, e)| raw_depth_only(&e.function, info.get_entry_point(i))),
        "depth profile supports textureLoad/textureDimensions only",
    )?;
    ensure(
        m.constants
            .iter()
            .all(|(_, c)| c.r#override == naga::Override::None),
        "override constants outside profile",
    )?;
    ensure(
        m.entry_points.len() == 2,
        "expected exactly two entry points",
    )?;
    let (vi, v) = m
        .entry_points
        .iter()
        .enumerate()
        .find(|(_, p)| p.name == vertex && p.stage == ShaderStage::Vertex)
        .ok_or("missing vertex entry")?;
    let (_, f) = m
        .entry_points
        .iter()
        .enumerate()
        .find(|(_, p)| p.name == fragment && p.stage == ShaderStage::Fragment)
        .ok_or("missing fragment entry")?;
    ensure(f.early_depth_test.is_none(), "early depth outside profile")?;
    let mut bindings = BTreeSet::new();
    for (handle, g) in m.global_variables.iter() {
        if let Some(b) = &g.binding {
            ensure(
                b.group == 0 && bindings.insert(b.binding),
                "bind group/layout",
            )?;
            let valid = match (b.binding, &m.types[g.ty].inner) {
                (
                    0,
                    TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class:
                            ImageClass::Sampled {
                                kind: ScalarKind::Float,
                                multi: false,
                            },
                    },
                ) => g.space == AddressSpace::Handle,
                (1, TypeInner::Sampler { comparison: false }) => g.space == AddressSpace::Handle,
                (2, _) => {
                    g.space == AddressSpace::Uniform && crate::uniform_check::params(&m, g.ty)
                }
                (
                    3,
                    TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Depth { multi: false },
                    },
                ) => profile.has_depth() && g.space == AddressSpace::Handle,
                (4, _) => {
                    profile.has_depth()
                        && g.space == AddressSpace::Uniform
                        && crate::uniform_check::camera(&m, g.ty)
                }
                (5, _) => {
                    profile.has_environment()
                        && g.space == AddressSpace::Uniform
                        && crate::uniform_check::environment(&m, g.ty)
                }
                (
                    6 | 7,
                    TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class:
                            ImageClass::Sampled {
                                kind: ScalarKind::Float,
                                multi: false,
                            },
                    },
                ) => geometry && g.space == AddressSpace::Handle,
                (
                    8,
                    TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Depth { multi: false },
                    },
                ) => shadow && g.space == AddressSpace::Handle,
                (9, _) => {
                    shadow
                        && g.space == AddressSpace::Uniform
                        && crate::uniform_check::shadow(&m, g.ty)
                }
                (
                    10,
                    TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class:
                            ImageClass::Sampled {
                                kind: ScalarKind::Float,
                                multi: false,
                            },
                    },
                ) => history && g.space == AddressSpace::Handle,
                (11, _) => {
                    history
                        && g.space == AddressSpace::Uniform
                        && crate::uniform_check::history(&m, g.ty)
                }
                (
                    12,
                    TypeInner::Image {
                        dim: ImageDimension::D2,
                        arrayed: false,
                        class:
                            ImageClass::Sampled {
                                kind: ScalarKind::Float,
                                multi: false,
                            },
                    },
                ) => light && g.space == AddressSpace::Handle,
                _ => false,
            };
            ensure(valid, "shader resource type")?;
            ensure(
                info.get_entry_point(vi)[handle].is_empty(),
                "resources are fragment-visible only",
            )?;
        } else {
            ensure(g.space == AddressSpace::Private, "unsupported global space")?;
        }
    }
    ensure(
        {
            let mut expected = BTreeSet::from([0, 1]);
            if bindings.contains(&2) {
                expected.insert(2);
            }
            if profile.has_depth() {
                expected.extend([3, 4]);
            }
            if profile.has_environment() {
                expected.insert(5);
            }
            if geometry {
                expected.extend([6, 7]);
            }
            if shadow {
                expected.extend([8, 9]);
            }
            if history {
                expected.extend([10, 11]);
            }
            if light {
                expected.insert(12);
            }
            bindings == expected
        },
        "required texture and sampler",
    )?;
    let args = &v.function.arguments;
    ensure(
        args.len() == 1
            && args[0].binding == Some(Binding::BuiltIn(BuiltIn::VertexIndex))
            && m.types[args[0].ty].inner
                == TypeInner::Scalar(Scalar {
                    kind: ScalarKind::Uint,
                    width: 4,
                }),
        "vertex input",
    )?;
    let vr = v.function.result.as_ref().ok_or("missing vertex result")?;
    let vo = io(&m, vr.ty, &vr.binding)?;
    ensure(
        vo.len() == 2
            && vo.iter().any(|(b, t)| {
                *b == Binding::BuiltIn(BuiltIn::Position { invariant: false })
                    && vec_float(&m, *t, VectorSize::Quad)
            })
            && vo
                .iter()
                .any(|(b, t)| uv(b) && vec_float(&m, *t, VectorSize::Bi)),
        "vertex output/link",
    )?;
    let mut fi = Vec::new();
    for a in &f.function.arguments {
        fi.extend(io(&m, a.ty, &a.binding)?);
    }
    ensure(
        fi.len() == 1 && uv(&fi[0].0) && vec_float(&m, fi[0].1, VectorSize::Bi),
        "fragment input/link",
    )?;
    let fr = f
        .function
        .result
        .as_ref()
        .ok_or("missing fragment output")?;
    let fo = io(&m, fr.ty, &fr.binding)?;
    ensure(
        fo.len() == 1
            && matches!(
                fo[0].0,
                Binding::Location {
                    location: 0,
                    second_blend_source: false,
                    ..
                }
            )
            && vec_float(&m, fo[0].1, VectorSize::Quad),
        "fragment attachment output",
    )?;
    Ok(bindings.contains(&2))
}
