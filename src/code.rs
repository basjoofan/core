use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

macro_rules! opcode {
    ($($name: ident => $code: literal,)+) => {
        pub enum Opcode {
            $(
                $name = $code,
            )+
        }

        impl From<u8> for Opcode {
            fn from(byte: u8) -> Self {
                match byte {
                    $(
                        $code => Opcode::$name,
                    )+
                    _ => panic!("Unknown opcode byte: {}", byte),
                }
            }
        }

        impl Opcode {
            pub fn code(&self) -> u8 {
                match self {
                    $(
                        Opcode::$name => $code,
                    )+
                }
            }

            pub fn width(&self) -> usize {
                match self {
                    Opcode::Constant => std::mem::size_of::<usize>(),
                    _ => usize::default(),
                }
            }
        }

        impl Display for Opcode {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                write!(
                    f,
                    "{}",
                    match self {
                        $(
                            Opcode::$name => stringify!($name),
                        )+
                    }
                )
            }
        }
    }
}

opcode! {
    Constant => 0,
    Add => 1,
}

pub fn make(opcode: Opcode, operand: Option<usize>) -> Vec<u8> {
    let mut instructions = Vec::default();
    instructions.push(opcode.code());
    match operand {
        Some(operand) => {
            for i in 0..opcode.width() {
                instructions.push((operand >> ((opcode.width() - i - 1) * 8)) as u8);
            }
        }
        _ => {}
    }
    instructions
}

pub fn read(bytes: &[u8]) -> usize {
    let mut value: usize = usize::default();
    for i in 0..bytes.len() {
        value |= (bytes[i] as usize) << ((bytes.len() - i - 1) * 8);
    }
    value
}

#[test]
fn test_make_instructions() {
    let tests = vec![
        (
            Opcode::Constant,
            Some(65534),
            vec![Opcode::Constant.code(), 0, 0, 0, 0, 0, 0, 255, 254],
        ),
        (Opcode::Add, None, vec![Opcode::Add.code()]),
    ];
    for (opcode, operand, expected) in tests {
        let actual = make(opcode, operand);
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_read_instructions() {
    let instructions = vec![
        make(Opcode::Add, None),
        make(Opcode::Constant, Some(2)),
        make(Opcode::Constant, Some(65535)),
    ]
    .concat();
    let expected = r#"000000 Add None
000001 Constant Some(2)
000010 Constant Some(65535)
"#;
    println!("{:?}", instructions);
    let mut actual = String::default();
    let mut i = 0;
    while i < instructions.len() {
        let opcode = Opcode::from(instructions[i]);
        let operand = match opcode.width() {
            0 => None,
            _ => Some(read(&instructions[i + 1..i + 1 + opcode.width()])),
        };
        let _ = actual.push_str(format!("{:06} {} {:?}\n", i, opcode, operand).as_str());
        i = i + 1 + opcode.width();
    }
    println!("{:?}", actual);
    assert_eq!(actual, expected);
}
