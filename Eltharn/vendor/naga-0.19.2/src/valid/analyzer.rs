

use super::{ExpressionError, FunctionError, ModuleInfo, ShaderStages, ValidationFlags};
use crate::span::{AddSpan as _, WithSpan};
use crate::{
    arena::{Arena, Handle},
    proc::{ResolveContext, TypeResolution},
};
use std::ops;

pub type NonUniformResult = Option<Handle<crate::Expression>>;

const DISABLE_UNIFORMITY_REQ_FOR_FRAGMENT_STAGE: bool = true;

bitflags::bitflags! {
    #[cfg_attr(feature = "serialize", derive(serde::Serialize))]
    #[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct UniformityRequirements: u8 {
        const WORK_GROUP_BARRIER = 0x1;
        const DERIVATIVE = if DISABLE_UNIFORMITY_REQ_FOR_FRAGMENT_STAGE { 0 } else { 0x2 };
        const IMPLICIT_LEVEL = if DISABLE_UNIFORMITY_REQ_FOR_FRAGMENT_STAGE { 0 } else { 0x4 };
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub struct Uniformity {
    pub non_uniform_result: NonUniformResult,
    pub requirements: UniformityRequirements,
}

impl Uniformity {
    const fn new() -> Self {
        Uniformity {
            non_uniform_result: None,
            requirements: UniformityRequirements::empty(),
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct ExitFlags: u8 {
        const MAY_RETURN = 0x1;
        const MAY_KILL = 0x2;
    }
}

struct FunctionUniformity {
    result: Uniformity,
    exit: ExitFlags,
}

impl ops::BitOr for FunctionUniformity {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        FunctionUniformity {
            result: Uniformity {
                non_uniform_result: self
                    .result
                    .non_uniform_result
                    .or(other.result.non_uniform_result),
                requirements: self.result.requirements | other.result.requirements,
            },
            exit: self.exit | other.exit,
        }
    }
}

impl FunctionUniformity {
    const fn new() -> Self {
        FunctionUniformity {
            result: Uniformity::new(),
            exit: ExitFlags::empty(),
        }
    }

    const fn exit_disruptor(&self) -> Option<UniformityDisruptor> {
        if self.exit.contains(ExitFlags::MAY_RETURN) {
            Some(UniformityDisruptor::Return)
        } else if self.exit.contains(ExitFlags::MAY_KILL) {
            Some(UniformityDisruptor::Discard)
        } else {
            None
        }
    }
}

bitflags::bitflags! {
    #[cfg_attr(feature = "serialize", derive(serde::Serialize))]
    #[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct GlobalUse: u8 {
        const READ = 0x1;
        const WRITE = 0x2;
        const QUERY = 0x4;
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub struct SamplingKey {
    pub image: Handle<crate::GlobalVariable>,
    pub sampler: Handle<crate::GlobalVariable>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub struct ExpressionInfo {
    pub uniformity: Uniformity,
    pub ref_count: usize,
    assignable_global: Option<Handle<crate::GlobalVariable>>,
    pub ty: TypeResolution,
}

impl ExpressionInfo {
    const fn new() -> Self {
        ExpressionInfo {
            uniformity: Uniformity::new(),
            ref_count: 0,
            assignable_global: None,
            ty: TypeResolution::Value(crate::TypeInner::Scalar(crate::Scalar {
                kind: crate::ScalarKind::Bool,
                width: 0,
            })),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
enum GlobalOrArgument {
    Global(Handle<crate::GlobalVariable>),
    Argument(u32),
}

impl GlobalOrArgument {
    fn from_expression(
        expression_arena: &Arena<crate::Expression>,
        expression: Handle<crate::Expression>,
    ) -> Result<GlobalOrArgument, ExpressionError> {
        Ok(match expression_arena[expression] {
            crate::Expression::GlobalVariable(var) => GlobalOrArgument::Global(var),
            crate::Expression::FunctionArgument(i) => GlobalOrArgument::Argument(i),
            crate::Expression::Access { base, .. }
            | crate::Expression::AccessIndex { base, .. } => match expression_arena[base] {
                crate::Expression::GlobalVariable(var) => GlobalOrArgument::Global(var),
                _ => return Err(ExpressionError::ExpectedGlobalOrArgument),
            },
            _ => return Err(ExpressionError::ExpectedGlobalOrArgument),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
struct Sampling {
    image: GlobalOrArgument,
    sampler: GlobalOrArgument,
}

#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub struct FunctionInfo {
    #[allow(dead_code)]
    flags: ValidationFlags,
    pub available_stages: ShaderStages,
    pub uniformity: Uniformity,
    pub may_kill: bool,

    pub sampling_set: crate::FastHashSet<SamplingKey>,

    global_uses: Box<[GlobalUse]>,

    expressions: Box<[ExpressionInfo]>,

    sampling: crate::FastHashSet<Sampling>,

    pub dual_source_blending: bool,
}

impl FunctionInfo {
    pub const fn global_variable_count(&self) -> usize {
        self.global_uses.len()
    }
    pub const fn expression_count(&self) -> usize {
        self.expressions.len()
    }
    pub fn dominates_global_use(&self, other: &Self) -> bool {
        for (self_global_uses, other_global_uses) in
            self.global_uses.iter().zip(other.global_uses.iter())
        {
            if !self_global_uses.contains(*other_global_uses) {
                return false;
            }
        }
        true
    }
}

impl ops::Index<Handle<crate::GlobalVariable>> for FunctionInfo {
    type Output = GlobalUse;
    fn index(&self, handle: Handle<crate::GlobalVariable>) -> &GlobalUse {
        &self.global_uses[handle.index()]
    }
}

impl ops::Index<Handle<crate::Expression>> for FunctionInfo {
    type Output = ExpressionInfo;
    fn index(&self, handle: Handle<crate::Expression>) -> &ExpressionInfo {
        &self.expressions[handle.index()]
    }
}

#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum UniformityDisruptor {
    #[error("Expression {0:?} produced non-uniform result, and control flow depends on it")]
    Expression(Handle<crate::Expression>),
    #[error("There is a Return earlier in the control flow of the function")]
    Return,
    #[error("There is a Discard earlier in the entry point across all called functions")]
    Discard,
}

impl FunctionInfo {
    #[must_use]
    fn add_ref_impl(
        &mut self,
        handle: Handle<crate::Expression>,
        global_use: GlobalUse,
    ) -> NonUniformResult {
        let info = &mut self.expressions[handle.index()];
        info.ref_count += 1;
        if let Some(global) = info.assignable_global {
            self.global_uses[global.index()] |= global_use;
        }
        info.uniformity.non_uniform_result
    }

    #[must_use]
    fn add_ref(&mut self, handle: Handle<crate::Expression>) -> NonUniformResult {
        self.add_ref_impl(handle, GlobalUse::READ)
    }

    #[must_use]
    fn add_assignable_ref(
        &mut self,
        handle: Handle<crate::Expression>,
        assignable_global: &mut Option<Handle<crate::GlobalVariable>>,
    ) -> NonUniformResult {
        let info = &mut self.expressions[handle.index()];
        info.ref_count += 1;
        if let Some(global) = info.assignable_global {
            if let Some(_old) = assignable_global.replace(global) {
                unreachable!()
            }
        }
        info.uniformity.non_uniform_result
    }

    fn process_call(
        &mut self,
        callee: &Self,
        arguments: &[Handle<crate::Expression>],
        expression_arena: &Arena<crate::Expression>,
    ) -> Result<FunctionUniformity, WithSpan<FunctionError>> {
        self.sampling_set
            .extend(callee.sampling_set.iter().cloned());
        for sampling in callee.sampling.iter() {
            let image_storage = match sampling.image {
                GlobalOrArgument::Global(var) => GlobalOrArgument::Global(var),
                GlobalOrArgument::Argument(i) => {
                    let handle = arguments[i as usize];
                    GlobalOrArgument::from_expression(expression_arena, handle).map_err(
                        |source| {
                            FunctionError::Expression { handle, source }
                                .with_span_handle(handle, expression_arena)
                        },
                    )?
                }
            };

            let sampler_storage = match sampling.sampler {
                GlobalOrArgument::Global(var) => GlobalOrArgument::Global(var),
                GlobalOrArgument::Argument(i) => {
                    let handle = arguments[i as usize];
                    GlobalOrArgument::from_expression(expression_arena, handle).map_err(
                        |source| {
                            FunctionError::Expression { handle, source }
                                .with_span_handle(handle, expression_arena)
                        },
                    )?
                }
            };

            match (image_storage, sampler_storage) {
                (GlobalOrArgument::Global(image), GlobalOrArgument::Global(sampler)) => {
                    self.sampling_set.insert(SamplingKey { image, sampler });
                }
                (image, sampler) => {
                    self.sampling.insert(Sampling { image, sampler });
                }
            }
        }

        for (mine, other) in self.global_uses.iter_mut().zip(callee.global_uses.iter()) {
            *mine |= *other;
        }

        Ok(FunctionUniformity {
            result: callee.uniformity.clone(),
            exit: if callee.may_kill {
                ExitFlags::MAY_KILL
            } else {
                ExitFlags::empty()
            },
        })
    }

    #[allow(clippy::or_fun_call)]
    fn process_expression(
        &mut self,
        handle: Handle<crate::Expression>,
        expression_arena: &Arena<crate::Expression>,
        other_functions: &[FunctionInfo],
        resolve_context: &ResolveContext,
        capabilities: super::Capabilities,
    ) -> Result<(), ExpressionError> {
        use crate::{Expression as E, SampleLevel as Sl};

        let expression = &expression_arena[handle];
        let mut assignable_global = None;
        let uniformity = match *expression {
            E::Access { base, index } => {
                let base_ty = self[base].ty.inner_with(resolve_context.types);

                let mut needed_caps = super::Capabilities::empty();
                let is_binding_array = match *base_ty {
                    crate::TypeInner::BindingArray {
                        base: array_element_ty_handle,
                        ..
                    } => {
                        let ub_st = super::Capabilities::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING;
                        let st_sb = super::Capabilities::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;
                        let sampler = super::Capabilities::SAMPLER_NON_UNIFORM_INDEXING;

                        let array_element_ty =
                            &resolve_context.types[array_element_ty_handle].inner;

                        needed_caps |= match *array_element_ty {
                            crate::TypeInner::Image { class, .. } => match class {
                                crate::ImageClass::Storage { .. } => ub_st,
                                _ => st_sb,
                            },
                            crate::TypeInner::Sampler { .. } => sampler,
                            _ => {
                                if let E::GlobalVariable(global_handle) = expression_arena[base] {
                                    let global = &resolve_context.global_vars[global_handle];
                                    match global.space {
                                        crate::AddressSpace::Uniform => ub_st,
                                        crate::AddressSpace::Storage { .. } => st_sb,
                                        _ => unreachable!(),
                                    }
                                } else {
                                    unreachable!()
                                }
                            }
                        };

                        true
                    }
                    _ => false,
                };

                if self[index].uniformity.non_uniform_result.is_some()
                    && !capabilities.contains(needed_caps)
                    && is_binding_array
                {
                    return Err(ExpressionError::MissingCapabilities(needed_caps));
                }

                Uniformity {
                    non_uniform_result: self
                        .add_assignable_ref(base, &mut assignable_global)
                        .or(self.add_ref(index)),
                    requirements: UniformityRequirements::empty(),
                }
            }
            E::AccessIndex { base, .. } => Uniformity {
                non_uniform_result: self.add_assignable_ref(base, &mut assignable_global),
                requirements: UniformityRequirements::empty(),
            },
            E::Splat { size: _, value } => Uniformity {
                non_uniform_result: self.add_ref(value),
                requirements: UniformityRequirements::empty(),
            },
            E::Swizzle { vector, .. } => Uniformity {
                non_uniform_result: self.add_ref(vector),
                requirements: UniformityRequirements::empty(),
            },
            E::Literal(_) | E::Constant(_) | E::ZeroValue(_) => Uniformity::new(),
            E::Compose { ref components, .. } => {
                let non_uniform_result = components
                    .iter()
                    .fold(None, |nur, &comp| nur.or(self.add_ref(comp)));
                Uniformity {
                    non_uniform_result,
                    requirements: UniformityRequirements::empty(),
                }
            }
            E::FunctionArgument(index) => {
                let arg = &resolve_context.arguments[index as usize];
                let uniform = match arg.binding {
                    Some(crate::Binding::BuiltIn(built_in)) => match built_in {
                        crate::BuiltIn::FrontFacing
                        | crate::BuiltIn::WorkGroupId
                        | crate::BuiltIn::WorkGroupSize
                        | crate::BuiltIn::NumWorkGroups => true,
                        _ => false,
                    },
                    Some(crate::Binding::Location {
                        interpolation: Some(crate::Interpolation::Flat),
                        ..
                    }) => true,
                    _ => false,
                };
                Uniformity {
                    non_uniform_result: if uniform { None } else { Some(handle) },
                    requirements: UniformityRequirements::empty(),
                }
            }
            E::GlobalVariable(gh) => {
                use crate::AddressSpace as As;
                assignable_global = Some(gh);
                let var = &resolve_context.global_vars[gh];
                let uniform = match var.space {
                    As::Function | As::Private => false,
                    As::WorkGroup => true,
                    As::Uniform | As::PushConstant => true,
                    As::Storage { access } => !access.contains(crate::StorageAccess::STORE),
                    As::Handle => false,
                };
                Uniformity {
                    non_uniform_result: if uniform { None } else { Some(handle) },
                    requirements: UniformityRequirements::empty(),
                }
            }
            E::LocalVariable(_) => Uniformity {
                non_uniform_result: Some(handle),
                requirements: UniformityRequirements::empty(),
            },
            E::Load { pointer } => Uniformity {
                non_uniform_result: self.add_ref(pointer),
                requirements: UniformityRequirements::empty(),
            },
            E::ImageSample {
                image,
                sampler,
                gather: _,
                coordinate,
                array_index,
                offset: _,
                level,
                depth_ref,
            } => {
                let image_storage = GlobalOrArgument::from_expression(expression_arena, image)?;
                let sampler_storage = GlobalOrArgument::from_expression(expression_arena, sampler)?;

                match (image_storage, sampler_storage) {
                    (GlobalOrArgument::Global(image), GlobalOrArgument::Global(sampler)) => {
                        self.sampling_set.insert(SamplingKey { image, sampler });
                    }
                    _ => {
                        self.sampling.insert(Sampling {
                            image: image_storage,
                            sampler: sampler_storage,
                        });
                    }
                }

                let array_nur = array_index.and_then(|h| self.add_ref(h));
                let level_nur = match level {
                    Sl::Auto | Sl::Zero => None,
                    Sl::Exact(h) | Sl::Bias(h) => self.add_ref(h),
                    Sl::Gradient { x, y } => self.add_ref(x).or(self.add_ref(y)),
                };
                let dref_nur = depth_ref.and_then(|h| self.add_ref(h));
                Uniformity {
                    non_uniform_result: self
                        .add_ref(image)
                        .or(self.add_ref(sampler))
                        .or(self.add_ref(coordinate))
                        .or(array_nur)
                        .or(level_nur)
                        .or(dref_nur),
                    requirements: if level.implicit_derivatives() {
                        UniformityRequirements::IMPLICIT_LEVEL
                    } else {
                        UniformityRequirements::empty()
                    },
                }
            }
            E::ImageLoad {
                image,
                coordinate,
                array_index,
                sample,
                level,
            } => {
                let array_nur = array_index.and_then(|h| self.add_ref(h));
                let sample_nur = sample.and_then(|h| self.add_ref(h));
                let level_nur = level.and_then(|h| self.add_ref(h));
                Uniformity {
                    non_uniform_result: self
                        .add_ref(image)
                        .or(self.add_ref(coordinate))
                        .or(array_nur)
                        .or(sample_nur)
                        .or(level_nur),
                    requirements: UniformityRequirements::empty(),
                }
            }
            E::ImageQuery { image, query } => {
                let query_nur = match query {
                    crate::ImageQuery::Size { level: Some(h) } => self.add_ref(h),
                    _ => None,
                };
                Uniformity {
                    non_uniform_result: self.add_ref_impl(image, GlobalUse::QUERY).or(query_nur),
                    requirements: UniformityRequirements::empty(),
                }
            }
            E::Unary { expr, .. } => Uniformity {
                non_uniform_result: self.add_ref(expr),
                requirements: UniformityRequirements::empty(),
            },
            E::Binary { left, right, .. } => Uniformity {
                non_uniform_result: self.add_ref(left).or(self.add_ref(right)),
                requirements: UniformityRequirements::empty(),
            },
            E::Select {
                condition,
                accept,
                reject,
            } => Uniformity {
                non_uniform_result: self
                    .add_ref(condition)
                    .or(self.add_ref(accept))
                    .or(self.add_ref(reject)),
                requirements: UniformityRequirements::empty(),
            },
            E::Derivative { expr, .. } => Uniformity {
                non_uniform_result: self.add_ref(expr),
                requirements: UniformityRequirements::DERIVATIVE,
            },
            E::Relational { argument, .. } => Uniformity {
                non_uniform_result: self.add_ref(argument),
                requirements: UniformityRequirements::empty(),
            },
            E::Math {
                fun: _,
                arg,
                arg1,
                arg2,
                arg3,
            } => {
                let arg1_nur = arg1.and_then(|h| self.add_ref(h));
                let arg2_nur = arg2.and_then(|h| self.add_ref(h));
                let arg3_nur = arg3.and_then(|h| self.add_ref(h));
                Uniformity {
                    non_uniform_result: self.add_ref(arg).or(arg1_nur).or(arg2_nur).or(arg3_nur),
                    requirements: UniformityRequirements::empty(),
                }
            }
            E::As { expr, .. } => Uniformity {
                non_uniform_result: self.add_ref(expr),
                requirements: UniformityRequirements::empty(),
            },
            E::CallResult(function) => other_functions[function.index()].uniformity.clone(),
            E::AtomicResult { .. } | E::RayQueryProceedResult => Uniformity {
                non_uniform_result: Some(handle),
                requirements: UniformityRequirements::empty(),
            },
            E::WorkGroupUniformLoadResult { .. } => Uniformity {
                non_uniform_result: None,
                requirements: UniformityRequirements::empty(),
            },
            E::ArrayLength(expr) => Uniformity {
                non_uniform_result: self.add_ref_impl(expr, GlobalUse::QUERY),
                requirements: UniformityRequirements::empty(),
            },
            E::RayQueryGetIntersection {
                query,
                committed: _,
            } => Uniformity {
                non_uniform_result: self.add_ref(query),
                requirements: UniformityRequirements::empty(),
            },
        };

        let ty = resolve_context.resolve(expression, |h| Ok(&self[h].ty))?;
        self.expressions[handle.index()] = ExpressionInfo {
            uniformity,
            ref_count: 0,
            assignable_global,
            ty,
        };
        Ok(())
    }

    #[allow(clippy::or_fun_call)]
    fn process_block(
        &mut self,
        statements: &crate::Block,
        other_functions: &[FunctionInfo],
        mut disruptor: Option<UniformityDisruptor>,
        expression_arena: &Arena<crate::Expression>,
    ) -> Result<FunctionUniformity, WithSpan<FunctionError>> {
        use crate::Statement as S;

        let mut combined_uniformity = FunctionUniformity::new();
        for statement in statements {
            let uniformity = match *statement {
                S::Emit(ref range) => {
                    let mut requirements = UniformityRequirements::empty();
                    for expr in range.clone() {
                        let req = self.expressions[expr.index()].uniformity.requirements;
                        if self
                            .flags
                            .contains(ValidationFlags::CONTROL_FLOW_UNIFORMITY)
                            && !req.is_empty()
                        {
                            if let Some(cause) = disruptor {
                                return Err(FunctionError::NonUniformControlFlow(req, expr, cause)
                                    .with_span_handle(expr, expression_arena));
                            }
                        }
                        requirements |= req;
                    }
                    FunctionUniformity {
                        result: Uniformity {
                            non_uniform_result: None,
                            requirements,
                        },
                        exit: ExitFlags::empty(),
                    }
                }
                S::Break | S::Continue => FunctionUniformity::new(),
                S::Kill => FunctionUniformity {
                    result: Uniformity::new(),
                    exit: if disruptor.is_some() {
                        ExitFlags::MAY_KILL
                    } else {
                        ExitFlags::empty()
                    },
                },
                S::Barrier(_) => FunctionUniformity {
                    result: Uniformity {
                        non_uniform_result: None,
                        requirements: UniformityRequirements::WORK_GROUP_BARRIER,
                    },
                    exit: ExitFlags::empty(),
                },
                S::WorkGroupUniformLoad { pointer, .. } => {
                    let _condition_nur = self.add_ref(pointer);


                    FunctionUniformity {
                        result: Uniformity {
                            non_uniform_result: None,
                            requirements: UniformityRequirements::WORK_GROUP_BARRIER,
                        },
                        exit: ExitFlags::empty(),
                    }
                }
                S::Block(ref b) => {
                    self.process_block(b, other_functions, disruptor, expression_arena)?
                }
                S::If {
                    condition,
                    ref accept,
                    ref reject,
                } => {
                    let condition_nur = self.add_ref(condition);
                    let branch_disruptor =
                        disruptor.or(condition_nur.map(UniformityDisruptor::Expression));
                    let accept_uniformity = self.process_block(
                        accept,
                        other_functions,
                        branch_disruptor,
                        expression_arena,
                    )?;
                    let reject_uniformity = self.process_block(
                        reject,
                        other_functions,
                        branch_disruptor,
                        expression_arena,
                    )?;
                    accept_uniformity | reject_uniformity
                }
                S::Switch {
                    selector,
                    ref cases,
                } => {
                    let selector_nur = self.add_ref(selector);
                    let branch_disruptor =
                        disruptor.or(selector_nur.map(UniformityDisruptor::Expression));
                    let mut uniformity = FunctionUniformity::new();
                    let mut case_disruptor = branch_disruptor;
                    for case in cases.iter() {
                        let case_uniformity = self.process_block(
                            &case.body,
                            other_functions,
                            case_disruptor,
                            expression_arena,
                        )?;
                        case_disruptor = if case.fall_through {
                            case_disruptor.or(case_uniformity.exit_disruptor())
                        } else {
                            branch_disruptor
                        };
                        uniformity = uniformity | case_uniformity;
                    }
                    uniformity
                }
                S::Loop {
                    ref body,
                    ref continuing,
                    break_if,
                } => {
                    let body_uniformity =
                        self.process_block(body, other_functions, disruptor, expression_arena)?;
                    let continuing_disruptor = disruptor.or(body_uniformity.exit_disruptor());
                    let continuing_uniformity = self.process_block(
                        continuing,
                        other_functions,
                        continuing_disruptor,
                        expression_arena,
                    )?;
                    if let Some(expr) = break_if {
                        let _ = self.add_ref(expr);
                    }
                    body_uniformity | continuing_uniformity
                }
                S::Return { value } => FunctionUniformity {
                    result: Uniformity {
                        non_uniform_result: value.and_then(|expr| self.add_ref(expr)),
                        requirements: UniformityRequirements::empty(),
                    },
                    exit: if disruptor.is_some() {
                        ExitFlags::MAY_RETURN
                    } else {
                        ExitFlags::empty()
                    },
                },
                S::Store { pointer, value } => {
                    let _ = self.add_ref_impl(pointer, GlobalUse::WRITE);
                    let _ = self.add_ref(value);
                    FunctionUniformity::new()
                }
                S::ImageStore {
                    image,
                    coordinate,
                    array_index,
                    value,
                } => {
                    let _ = self.add_ref_impl(image, GlobalUse::WRITE);
                    if let Some(expr) = array_index {
                        let _ = self.add_ref(expr);
                    }
                    let _ = self.add_ref(coordinate);
                    let _ = self.add_ref(value);
                    FunctionUniformity::new()
                }
                S::Call {
                    function,
                    ref arguments,
                    result: _,
                } => {
                    for &argument in arguments {
                        let _ = self.add_ref(argument);
                    }
                    let info = &other_functions[function.index()];
                    self.process_call(info, arguments, expression_arena)?
                }
                S::Atomic {
                    pointer,
                    ref fun,
                    value,
                    result: _,
                } => {
                    let _ = self.add_ref_impl(pointer, GlobalUse::WRITE);
                    let _ = self.add_ref(value);
                    if let crate::AtomicFunction::Exchange { compare: Some(cmp) } = *fun {
                        let _ = self.add_ref(cmp);
                    }
                    FunctionUniformity::new()
                }
                S::RayQuery { query, ref fun } => {
                    let _ = self.add_ref(query);
                    if let crate::RayQueryFunction::Initialize {
                        acceleration_structure,
                        descriptor,
                    } = *fun
                    {
                        let _ = self.add_ref(acceleration_structure);
                        let _ = self.add_ref(descriptor);
                    }
                    FunctionUniformity::new()
                }
            };

            disruptor = disruptor.or(uniformity.exit_disruptor());
            combined_uniformity = combined_uniformity | uniformity;
        }
        Ok(combined_uniformity)
    }
}

impl ModuleInfo {
    pub(super) fn process_const_expression(
        &mut self,
        handle: Handle<crate::Expression>,
        resolve_context: &ResolveContext,
        gctx: crate::proc::GlobalCtx,
    ) -> Result<(), super::ConstExpressionError> {
        self.const_expression_types[handle.index()] =
            resolve_context.resolve(&gctx.const_expressions[handle], |h| Ok(&self[h]))?;
        Ok(())
    }

    pub(super) fn process_function(
        &self,
        fun: &crate::Function,
        module: &crate::Module,
        flags: ValidationFlags,
        capabilities: super::Capabilities,
    ) -> Result<FunctionInfo, WithSpan<FunctionError>> {
        let mut info = FunctionInfo {
            flags,
            available_stages: ShaderStages::all(),
            uniformity: Uniformity::new(),
            may_kill: false,
            sampling_set: crate::FastHashSet::default(),
            global_uses: vec![GlobalUse::empty(); module.global_variables.len()].into_boxed_slice(),
            expressions: vec![ExpressionInfo::new(); fun.expressions.len()].into_boxed_slice(),
            sampling: crate::FastHashSet::default(),
            dual_source_blending: false,
        };
        let resolve_context =
            ResolveContext::with_locals(module, &fun.local_variables, &fun.arguments);

        for (handle, _) in fun.expressions.iter() {
            if let Err(source) = info.process_expression(
                handle,
                &fun.expressions,
                &self.functions,
                &resolve_context,
                capabilities,
            ) {
                return Err(FunctionError::Expression { handle, source }
                    .with_span_handle(handle, &fun.expressions));
            }
        }

        for (_, expr) in fun.local_variables.iter() {
            if let Some(init) = expr.init {
                let _ = info.add_ref(init);
            }
        }

        let uniformity = info.process_block(&fun.body, &self.functions, None, &fun.expressions)?;
        info.uniformity = uniformity.result;
        info.may_kill = uniformity.exit.contains(ExitFlags::MAY_KILL);

        Ok(info)
    }

    pub fn get_entry_point(&self, index: usize) -> &FunctionInfo {
        &self.entry_points[index]
    }
}


