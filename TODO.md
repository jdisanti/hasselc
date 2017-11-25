Compiler TODO
-------------

### Minimum viable product

- [X] String constants
- [ ] CLI for compiling arbitrary files
- [ ] Configurable runtime environment
- [ ] Conditions: && and ||

### Other items

- [ ] Error Handling
  - [X] Type checking
  - [X] Fix all error handling unimplemented!() and TODOs
  - [ ] Check for return in functions that have return type
  - [ ] Verify break keywords are inside of loops only
- [ ] Bug fixes
  - [ ] If a void function doesn't have a return statement, generate a RTS instruction at the end
- [X] Constants
- [X] Arrays and pointers
- [ ] 16-bit numbers
  - [X] Return/assign support
  - [ ] Addition/subtraction
  - [ ] Multiplication/division
  - [ ] 16-bit value arrays
- [ ] Multiply and divide
- [ ] Break out of loops with break
- [ ] Optimization
  - [ ] Constant evaluation for binary operators in IR
  - [ ] For functions with 0 frame size, don't modify the stack pointer
  - [ ] For comparisons in a condition, generate smarter code
  - [ ] Use Y register in loops somehow
  - [ ] CLC + ADC #1 -> INC
  - [ ] SEC + SBC #1 -> DEC
  - [ ] LDY imm + STA addr,Y -> STA addr + imm
  - [ ] Peephole: Change load/store of absolute address in zero page to use faster zero page access
