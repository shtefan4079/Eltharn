use super::{recyclable::Recyclable, Instruction, LogicalLayout, PhysicalLayout};
use spirv::{Op, Word, MAGIC_NUMBER};
use std::iter;

const GENERATOR: Word = 28;

impl PhysicalLayout {
    pub(super) const fn new(version: Word) -> Self {
        PhysicalLayout {
            magic_number: MAGIC_NUMBER,
            version,
            generator: GENERATOR,
            bound: 0,
            instruction_schema: 0x0u32,
        }
    }

    pub(super) fn in_words(&self, sink: &mut impl Extend<Word>) {
        sink.extend(iter::once(self.magic_number));
        sink.extend(iter::once(self.version));
        sink.extend(iter::once(self.generator));
        sink.extend(iter::once(self.bound));
        sink.extend(iter::once(self.instruction_schema));
    }
}

impl Recyclable for PhysicalLayout {
    fn recycle(self) -> Self {
        PhysicalLayout {
            magic_number: self.magic_number,
            version: self.version,
            generator: self.generator,
            instruction_schema: self.instruction_schema,
            bound: 0,
        }
    }
}

impl LogicalLayout {
    pub(super) fn in_words(&self, sink: &mut impl Extend<Word>) {
        sink.extend(self.capabilities.iter().cloned());
        sink.extend(self.extensions.iter().cloned());
        sink.extend(self.ext_inst_imports.iter().cloned());
        sink.extend(self.memory_model.iter().cloned());
        sink.extend(self.entry_points.iter().cloned());
        sink.extend(self.execution_modes.iter().cloned());
        sink.extend(self.debugs.iter().cloned());
        sink.extend(self.annotations.iter().cloned());
        sink.extend(self.declarations.iter().cloned());
        sink.extend(self.function_declarations.iter().cloned());
        sink.extend(self.function_definitions.iter().cloned());
    }
}

impl Recyclable for LogicalLayout {
    fn recycle(self) -> Self {
        Self {
            capabilities: Recyclable::recycle(self.capabilities),
            extensions: Recyclable::recycle(self.extensions),
            ext_inst_imports: Recyclable::recycle(self.ext_inst_imports),
            memory_model: Recyclable::recycle(self.memory_model),
            entry_points: Recyclable::recycle(self.entry_points),
            execution_modes: Recyclable::recycle(self.execution_modes),
            debugs: Recyclable::recycle(self.debugs),
            annotations: Recyclable::recycle(self.annotations),
            declarations: Recyclable::recycle(self.declarations),
            function_declarations: Recyclable::recycle(self.function_declarations),
            function_definitions: Recyclable::recycle(self.function_definitions),
        }
    }
}

impl Instruction {
    pub(super) const fn new(op: Op) -> Self {
        Instruction {
            op,
            wc: 1,                                                   
            type_id: None,
            result_id: None,
            operands: vec![],
        }
    }

    #[allow(clippy::panic)]
    pub(super) fn set_type(&mut self, id: Word) {
        assert!(self.type_id.is_none(), "Type can only be set once");
        self.type_id = Some(id);
        self.wc += 1;
    }

    #[allow(clippy::panic)]
    pub(super) fn set_result(&mut self, id: Word) {
        assert!(self.result_id.is_none(), "Result can only be set once");
        self.result_id = Some(id);
        self.wc += 1;
    }

    pub(super) fn add_operand(&mut self, operand: Word) {
        self.operands.push(operand);
        self.wc += 1;
    }

    pub(super) fn add_operands(&mut self, operands: Vec<Word>) {
        for operand in operands.into_iter() {
            self.add_operand(operand)
        }
    }

    pub(super) fn to_words(&self, sink: &mut impl Extend<Word>) {
        sink.extend(Some(self.wc << 16 | self.op as u32));
        sink.extend(self.type_id);
        sink.extend(self.result_id);
        sink.extend(self.operands.iter().cloned());
    }
}


