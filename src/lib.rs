use async_std::{
    fs::{create_dir_all, File},
    io::{prelude::BufReadExt, BufReader},
    path::PathBuf,
};
use std::str::FromStr;

/// 从命令行参数解析配置文件地址
pub async fn path_from_args(n: usize) -> PathBuf {
    if let Some(path) = std::env::args()
        .nth(n)
        .and_then(|path| PathBuf::from_str(&path).ok())
    {
        if path.is_dir().await || create_dir_all(&path).await.is_ok() {
            return path;
        }
    }
    std::env::current_exe().unwrap().parent().unwrap().into()
}

/// 读取配置文件
pub async fn read(path: PathBuf, mut f: impl FnMut((String, String))) {
    if let Ok(file) = File::open(path).await {
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        loop {
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    parse_line(line.trim_end()).map(&mut f);
                    line.clear();
                }
            }
        }
    }
}

/// 解析配置文件行
fn parse_line(line: &str) -> Option<(String, String)> {
    let mut key = String::new();
    let mut value = String::new();
    let mut step = 0;
    for it in line.as_bytes().iter().take_while(|c| **c != b'#') {
        match step {
            0 => match *it {
                b'|' => break,
                b' ' => {}
                c => {
                    key.push(c as char);
                    step += 1;
                }
            },
            1 => match *it {
                b'|' => step += 2,
                b' ' => step += 1,
                c => key.push(c as char),
            },
            2 => match *it {
                b'|' => step += 1,
                b' ' => {}
                c => {
                    key.push(' ');
                    key.push(c as char);
                    step -= 1;
                }
            },
            3 => match *it {
                b' ' => {}
                c => {
                    value.push(c as char);
                    step += 1;
                }
            },
            4 => match *it {
                b' ' => step += 1,
                c => value.push(c as char),
            },
            5 => match *it {
                b' ' => {}
                c => {
                    value.push(' ');
                    value.push(c as char);
                    step -= 1;
                }
            },
            _ => unreachable!(),
        }
    }
    Some((key, value)).filter(|_| step != 0)
}

#[test]
fn test_parse_line() {
    assert_eq!(None, parse_line(""));
    assert_eq!(None, parse_line("# comment"));
    assert_eq!(None, parse_line(" | key is empty"));
    assert_eq!(
        Some(("key".into(), "\"key\"".into())),
        parse_line("key | \"key\"")
    );
    assert_eq!(
        Some(("key".into(), "".into())),
        parse_line("key | # value is empty")
    );
    assert_eq!(
        Some(("1 + 1".into(), "= 2".into())),
        parse_line("   1   +   1 | =     2")
    );
}
