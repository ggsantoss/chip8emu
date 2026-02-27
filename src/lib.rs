use rand::random;

const RAM_SIZE: usize = 4096;
pub const SCREEN_WIDTH: usize = 64; // 64x32
pub const SCREEN_HEIGHT: usize = 32;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const FONTSET_SIZE: usize = 80;

const FONTSET: [u8;FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Emu {
    pc: u16, // The program starts here, 0x200
    ram: [u8;RAM_SIZE],
    screen: [bool;SCREEN_WIDTH * SCREEN_HEIGHT], // Bool because there are just two colors, black and white
    v_reg: [u8;NUM_REGS], // VO - VF (0-15 im hexadecimal) give instructions to the CPU (opcodes)
    i_reg: u16, // Used for indexing
    sp: u16, // Pointer to indicate where in the stack (VecDeque can be used too)
    stack: [u16;STACK_SIZE], // Can only be written or read by LIFO
    keys: [bool;NUM_KEYS],
    dt: u8, // typical timer 
    st: u8, // Emit sound
}

const START_ADDR: u16 = 0x200;


// Initializes everything with a constructor, very straightfoward.
impl Emu {
    pub fn new() -> Self {
        let mut new_emu = Self {
            pc: START_ADDR,
            ram: [0;RAM_SIZE],
            screen: [false;SCREEN_WIDTH * SCREEN_HEIGHT],
            v_reg: [0;NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0;STACK_SIZE],
            keys: [false;NUM_KEYS],
            dt: 0,
            st: 0,
        };

        new_emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET); // Loading font into RAM
        new_emu
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;
    }
    
    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = (START_ADDR as usize) + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }

    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn tick(&mut self){
        let op = self.fetch();
        self.execute(op);
    }

fn execute(&mut self, op: u16) {
    let digit1 = (op & 0xF000) >> 12;
    let digit2 = (op & 0x0F00) >> 8;
    let digit3 = (op & 0x00F0) >> 4;
    let digit4 = op & 0x000F;

    match (digit1, digit2, digit3, digit4) {

        // 0000 - NOP
        (0, 0, 0, 0) => return,

        // 00E0 - Clear screen
        (0, 0, 0xE, 0) => {
            self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        },

        // 00EE - Return from subroutine
        (0, 0, 0xE, 0xE) => {
            let ret_addr = self.pop();
            self.pc = ret_addr;
        },

        // 1NNN - Jump to address NNN
        (1, _, _, _) => {
            let nnn = op & 0xFFF;
            self.pc = nnn;
        },

        // 2NNN - Call subroutine at NNN
        (2, _, _, _) => {
            let nnn = op & 0xFFF;
            self.push(self.pc);
            self.pc = nnn;
        },

        // 3XNN - Skip if Vx == NN
        (3, _, _, _) => {
            let x = digit2 as usize;
            let nn = (op & 0xFF) as u8;
            if self.v_reg[x] == nn {
                self.pc += 2;
            }
        },

        // 4XNN - Skip if Vx != NN
        (4, _, _, _) => {
            let x = digit2 as usize;
            let nn = (op & 0xFF) as u8;
            if self.v_reg[x] != nn {
                self.pc += 2;
            }
        },

        // 5XY0 - Skip if Vx == Vy
        (5, _, _, 0) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            if self.v_reg[x] == self.v_reg[y] {
                self.pc += 2;
            }
        },

        // 6XNN - Set Vx = NN
        (6, _, _, _) => {
            let x = digit2 as usize;
            let nn = (op & 0xFF) as u8;
            self.v_reg[x] = nn;
        },

        // 7XNN - Set Vx = Vx + NN (no carry)
        (7, _, _, _) => {
            let x = digit2 as usize;
            let nn = (op & 0xFF) as u8;
            self.v_reg[x] = self.v_reg[x].wrapping_add(nn);
        },

        // 8XY0 - Set Vx = Vy
        (8, _, _, 0) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            self.v_reg[x] = self.v_reg[y];
        },

        // 8XY1 - Set Vx = Vx OR Vy
        (8, _, _, 1) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            self.v_reg[x] |= self.v_reg[y];
        },

        // 8XY2 - Set Vx = Vx AND Vy
        (8, _, _, 2) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            self.v_reg[x] &= self.v_reg[y];
        },

        // 8XY3 - Set Vx = Vx XOR Vy
        (8, _, _, 3) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            self.v_reg[x] ^= self.v_reg[y];
        },

        // 8XY4 - Set Vx = Vx + Vy, VF = carry
        (8, _, _, 4) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            let (new_vx, carry) = self.v_reg[x].overflowing_add(self.v_reg[y]);
            let new_vf = if carry { 1 } else { 0 };
            self.v_reg[x] = new_vx;
            self.v_reg[0xF] = new_vf;
        },

        // 8XY5 - Set Vx = Vx - Vy, VF = NOT borrow
        (8, _, _, 5) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            let (new_vx, borrow) = self.v_reg[x].overflowing_sub(self.v_reg[y]);
            let new_vf = if borrow { 0 } else { 1 };
            self.v_reg[x] = new_vx;
            self.v_reg[0xF] = new_vf;
        },

        // 8XY6 - Set Vx = Vx SHR 1, VF = LSB
        (8, _, _, 6) => {
            let x = digit2 as usize;
            let lsb = self.v_reg[x] & 1;
            self.v_reg[x] >>= 1;
            self.v_reg[0xF] = lsb;
        },

        // 8XY7 - Set Vx = Vy - Vx, VF = NOT borrow
        (8, _, _, 7) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            let (new_vx, borrow) = self.v_reg[y].overflowing_sub(self.v_reg[x]);
            let new_vf = if borrow { 0 } else { 1 };
            self.v_reg[x] = new_vx;
            self.v_reg[0xF] = new_vf;
        },

        // 8XYE - Set Vx = Vx SHL 1, VF = MSB
        (8, _, _, 0xE) => {
            let x = digit2 as usize;
            let msb = (self.v_reg[x] >> 7) & 1;
            self.v_reg[x] <<= 1;
            self.v_reg[0xF] = msb;
        },

        // 9XY0 - Skip if Vx != Vy
        (9, _, _, 0) => {
            let x = digit2 as usize;
            let y = digit3 as usize;
            if self.v_reg[x] != self.v_reg[y] {
                self.pc += 2;
            }
        },

        // ANNN - Set I = NNN
        (0xA, _, _, _) => {
            let nnn = op & 0xFFF;
            self.i_reg = nnn;
        },

        // BNNN - Jump to V0 + NNN
        (0xB, _, _, _) => {
            let nnn = op & 0xFFF;
            self.pc = (self.v_reg[0] as u16) + nnn;
        },

        // CXNN - Set Vx = random byte AND NN
        (0xC, _, _, _) => {
            let x = digit2 as usize;
            let nn = (op & 0xFF) as u8;
            let rng: u8 = random();
            self.v_reg[x] = rng & nn;
        },

        // DXYN - Draw sprite at (Vx, Vy), N rows tall
        (0xD, _, _, _) => {
            let x_coord = self.v_reg[digit2 as usize] as u16;
            let y_coord = self.v_reg[digit3 as usize] as u16;
            let num_rows = digit4;
            let mut flipped = false;

            for y_line in 0..num_rows {
                let addr = self.i_reg + y_line as u16;
                let pixels = self.ram[addr as usize];
                for x_line in 0..8u16 {
                    if (pixels & (0b1000_0000 >> x_line)) != 0 {
                        let x = (x_coord + x_line) as usize % SCREEN_WIDTH;
                        let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;
                        let idx = x + SCREEN_WIDTH * y;
                        flipped |= self.screen[idx];
                        self.screen[idx] ^= true;
                    }
                }
            }

            self.v_reg[0xF] = if flipped { 1 } else { 0 };
        },

        // EX9E - Skip if key Vx is pressed
        (0xE, _, 9, 0xE) => {
            let x = digit2 as usize;
            let vx = self.v_reg[x];
            if self.keys[vx as usize] {
                self.pc += 2;
            }
        },

        // EXA1 - Skip if key Vx is NOT pressed
        (0xE, _, 0xA, 1) => {
            let x = digit2 as usize;
            let vx = self.v_reg[x];
            if !self.keys[vx as usize] {
                self.pc += 2;
            }
        },

        // FX07 - Set Vx = delay timer
        (0xF, _, 0, 7) => {
            let x = digit2 as usize;
            self.v_reg[x] = self.dt;
        },

        // FX0A - Wait for key press, store in Vx
        (0xF, _, 0, 0xA) => {
            let x = digit2 as usize;
            let mut pressed = false;
            for i in 0..self.keys.len() {
                if self.keys[i] {
                    self.v_reg[x] = i as u8;
                    pressed = true;
                    break;
                }
            }
            if !pressed {
                self.pc -= 2;
            }
        },

        // FX15 - Set delay timer = Vx
        (0xF, _, 1, 5) => {
            let x = digit2 as usize;
            self.dt = self.v_reg[x];
        },

        // FX18 - Set sound timer = Vx
        (0xF, _, 1, 8) => {
            let x = digit2 as usize;
            self.st = self.v_reg[x];
        },

        // FX1E - Set I = I + Vx
        (0xF, _, 1, 0xE) => {
            let x = digit2 as usize;
            let vx = self.v_reg[x] as u16;
            self.i_reg = self.i_reg.wrapping_add(vx);
        },

        // FX29 - Set I = address of font sprite for digit Vx
        (0xF, _, 2, 9) => {
            let x = digit2 as usize;
            let c = self.v_reg[x] as u16;
            self.i_reg = c * 5;
        },

        // FX33 - Store BCD of Vx in I, I+1, I+2
        (0xF, _, 3, 3) => {
            let x = digit2 as usize;
            let vx = self.v_reg[x] as f32;
            let hundreds = (vx / 100.0).floor() as u8;
            let tens = ((vx / 10.0) % 10.0).floor() as u8;
            let ones = (vx % 10.0) as u8;
            self.ram[self.i_reg as usize]     = hundreds;
            self.ram[self.i_reg as usize + 1] = tens;
            self.ram[self.i_reg as usize + 2] = ones;
        },

        // FX55 - Store V0-Vx in memory starting at I
        (0xF, _, 5, 5) => {
            let x = digit2 as usize;
            let i = self.i_reg as usize;
            for idx in 0..=x {
                self.ram[i + idx] = self.v_reg[idx];
            }
        },

        // FX65 - Load V0-Vx from memory starting at I
        (0xF, _, 6, 5) => {
            let x = digit2 as usize;
            let i = self.i_reg as usize;
            for idx in 0..=x {
                self.v_reg[idx] = self.ram[i + idx];
            }
        },

        (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", op),
    }
}
    // Fetch to 0x200
    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;
        let op = (higher_byte << 8) | lower_byte;
        self.pc += 2;
        op
    }

    pub fn tick_timers(&mut self){
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            if self.st == 1 {
                // BEEP
            }
            self.st -= 1;
        }
    }
}
