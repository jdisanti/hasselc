//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use std::sync::{Arc, RwLock};

use llir::{RunBlock, Statement};
use symbol_table::{SymbolRef, SymbolTable};

pub type BlockRef = usize;

pub struct BlockBuilder<'a> {
    pub block_ref: BlockRef,
    block: &'a mut RunBlock,
}

impl<'a> BlockBuilder<'a> {
    pub fn add_statement(&mut self, statement: Statement) -> &mut Self {
        self.block.statements.push(statement);
        self
    }

    pub fn symbol(&self) -> SymbolRef {
        self.block.symbol
    }
}

pub struct RunBuilder {
    symbol_table: Arc<RwLock<SymbolTable>>,
    blocks: Vec<RunBlock>,
    current_block: BlockRef,
}

impl RunBuilder {
    pub fn new(symbol_table: Arc<RwLock<SymbolTable>>) -> RunBuilder {
        let mut run_builder = RunBuilder {
            symbol_table: symbol_table,
            blocks: Vec::new(),
            current_block: 0,
        };
        run_builder.new_block();
        run_builder
    }

    pub fn symbol_table(&self) -> &Arc<RwLock<SymbolTable>> {
        &self.symbol_table
    }

    pub fn current_block<'a>(&'a mut self) -> BlockBuilder<'a> {
        BlockBuilder {
            block_ref: self.current_block,
            block: &mut self.blocks[self.current_block],
        }
    }

    pub fn new_block<'a>(&'a mut self) -> BlockBuilder<'a> {
        let (block_name, block_ref) = self.symbol_table.write().unwrap().new_block_name();
        let block = RunBlock::new(block_name, block_ref);
        self.blocks.push(block);
        self.current_block = self.blocks.len() - 1;
        BlockBuilder {
            block_ref: self.current_block,
            block: &mut self.blocks[self.current_block],
        }
    }

    pub fn block<'a>(&'a mut self, block_ref: BlockRef) -> BlockBuilder<'a> {
        BlockBuilder {
            block_ref: block_ref,
            block: &mut self.blocks[block_ref],
        }
    }

    /// Returns the index of the first appended block
    pub fn append_blocks(&mut self, blocks: Vec<RunBlock>) -> BlockRef {
        let first_appended = self.blocks.len();
        self.blocks.extend(blocks.into_iter());
        self.current_block = self.blocks.len() - 1;
        first_appended
    }

    pub fn build(self) -> Vec<RunBlock> {
        self.blocks
    }
}
