
use crate::proc::GlobalCtx;
use crate::Handle;

impl crate::proc::TypeResolution {
    pub fn to_wgsl(&self, gctx: &GlobalCtx) -> String {
        match *self {
            crate::proc::TypeResolution::Handle(handle) => handle.to_wgsl(gctx),
            crate::proc::TypeResolution::Value(ref inner) => inner.to_wgsl(gctx),
        }
    }
}

impl Handle<crate::Type> {
    pub fn to_wgsl(self, gctx: &GlobalCtx) -> String {
        let ty = &gctx.types[self];
        match ty.name {
            Some(ref name) => name.clone(),
            None => ty.inner.to_wgsl(gctx),
        }
    }
}

impl crate::TypeInner {
    pub fn to_wgsl(&self, gctx: &GlobalCtx) -> String {
        use crate::TypeInner as Ti;

        match *self {
            Ti::Scalar(scalar) => scalar.to_wgsl(),
            Ti::Vector { size, scalar } => {
                format!("vec{}<{}>", size as u32, scalar.to_wgsl())
            }
            Ti::Matrix {
                columns,
                rows,
                scalar,
            } => {
                format!(
                    "mat{}x{}<{}>",
                    columns as u32,
                    rows as u32,
                    scalar.to_wgsl(),
                )
            }
            Ti::Atomic(scalar) => {
                format!("atomic<{}>", scalar.to_wgsl())
            }
            Ti::Pointer { base, .. } => {
                let name = base.to_wgsl(gctx);
                format!("ptr<{name}>")
            }
            Ti::ValuePointer { scalar, .. } => {
                format!("ptr<{}>", scalar.to_wgsl())
            }
            Ti::Array { base, size, .. } => {
                let base = base.to_wgsl(gctx);
                match size {
                    crate::ArraySize::Constant(size) => format!("array<{base}, {size}>"),
                    crate::ArraySize::Dynamic => format!("array<{base}>"),
                }
            }
            Ti::Struct { .. } => {
                "struct".to_string()
            }
            Ti::Image {
                dim,
                arrayed,
                class,
            } => {
                let dim_suffix = match dim {
                    crate::ImageDimension::D1 => "_1d",
                    crate::ImageDimension::D2 => "_2d",
                    crate::ImageDimension::D3 => "_3d",
                    crate::ImageDimension::Cube => "_cube",
                };
                let array_suffix = if arrayed { "_array" } else { "" };

                let class_suffix = match class {
                    crate::ImageClass::Sampled { multi: true, .. } => "_multisampled",
                    crate::ImageClass::Depth { multi: false } => "_depth",
                    crate::ImageClass::Depth { multi: true } => "_depth_multisampled",
                    crate::ImageClass::Sampled { multi: false, .. }
                    | crate::ImageClass::Storage { .. } => "",
                };

                let type_in_brackets = match class {
                    crate::ImageClass::Sampled { kind, .. } => {
                        let element_type = crate::Scalar { kind, width: 4 }.to_wgsl();
                        format!("<{element_type}>")
                    }
                    crate::ImageClass::Depth { multi: _ } => String::new(),
                    crate::ImageClass::Storage { format, access } => {
                        if access.contains(crate::StorageAccess::STORE) {
                            format!("<{},write>", format.to_wgsl())
                        } else {
                            format!("<{}>", format.to_wgsl())
                        }
                    }
                };

                format!("texture{class_suffix}{dim_suffix}{array_suffix}{type_in_brackets}")
            }
            Ti::Sampler { .. } => "sampler".to_string(),
            Ti::AccelerationStructure => "acceleration_structure".to_string(),
            Ti::RayQuery => "ray_query".to_string(),
            Ti::BindingArray { base, size, .. } => {
                let member_type = &gctx.types[base];
                let base = member_type.name.as_deref().unwrap_or("unknown");
                match size {
                    crate::ArraySize::Constant(size) => format!("binding_array<{base}, {size}>"),
                    crate::ArraySize::Dynamic => format!("binding_array<{base}>"),
                }
            }
        }
    }
}

impl crate::Scalar {
    pub fn to_wgsl(self) -> String {
        let prefix = match self.kind {
            crate::ScalarKind::Sint => "i",
            crate::ScalarKind::Uint => "u",
            crate::ScalarKind::Float => "f",
            crate::ScalarKind::Bool => return "bool".to_string(),
            crate::ScalarKind::AbstractInt => return "{AbstractInt}".to_string(),
            crate::ScalarKind::AbstractFloat => return "{AbstractFloat}".to_string(),
        };
        format!("{}{}", prefix, self.width * 8)
    }
}

impl crate::StorageFormat {
    pub const fn to_wgsl(self) -> &'static str {
        use crate::StorageFormat as Sf;
        match self {
            Sf::R8Unorm => "r8unorm",
            Sf::R8Snorm => "r8snorm",
            Sf::R8Uint => "r8uint",
            Sf::R8Sint => "r8sint",
            Sf::R16Uint => "r16uint",
            Sf::R16Sint => "r16sint",
            Sf::R16Float => "r16float",
            Sf::Rg8Unorm => "rg8unorm",
            Sf::Rg8Snorm => "rg8snorm",
            Sf::Rg8Uint => "rg8uint",
            Sf::Rg8Sint => "rg8sint",
            Sf::R32Uint => "r32uint",
            Sf::R32Sint => "r32sint",
            Sf::R32Float => "r32float",
            Sf::Rg16Uint => "rg16uint",
            Sf::Rg16Sint => "rg16sint",
            Sf::Rg16Float => "rg16float",
            Sf::Rgba8Unorm => "rgba8unorm",
            Sf::Rgba8Snorm => "rgba8snorm",
            Sf::Rgba8Uint => "rgba8uint",
            Sf::Rgba8Sint => "rgba8sint",
            Sf::Bgra8Unorm => "bgra8unorm",
            Sf::Rgb10a2Uint => "rgb10a2uint",
            Sf::Rgb10a2Unorm => "rgb10a2unorm",
            Sf::Rg11b10Float => "rg11b10float",
            Sf::Rg32Uint => "rg32uint",
            Sf::Rg32Sint => "rg32sint",
            Sf::Rg32Float => "rg32float",
            Sf::Rgba16Uint => "rgba16uint",
            Sf::Rgba16Sint => "rgba16sint",
            Sf::Rgba16Float => "rgba16float",
            Sf::Rgba32Uint => "rgba32uint",
            Sf::Rgba32Sint => "rgba32sint",
            Sf::Rgba32Float => "rgba32float",
            Sf::R16Unorm => "r16unorm",
            Sf::R16Snorm => "r16snorm",
            Sf::Rg16Unorm => "rg16unorm",
            Sf::Rg16Snorm => "rg16snorm",
            Sf::Rgba16Unorm => "rgba16unorm",
            Sf::Rgba16Snorm => "rgba16snorm",
        }
    }
}

