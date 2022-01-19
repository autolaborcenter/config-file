use async_std::{
    fs::{create_dir_all, File},
    io::{prelude::BufReadExt, BufReader},
    path::PathBuf,
    task,
};
use std::str::FromStr;

pub struct ConfigFile(BufReader<File>);

impl ConfigFile {
    #[inline]
    pub async fn from_args(n: usize, file_name: &str) -> Option<(PathBuf, Self)> {
        let path = path_from_args(n).await;
        File::open(path.join(file_name))
            .await
            .ok()
            .map(|file| (path, Self(BufReader::new(file))))
    }
}

impl Iterator for ConfigFile {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        loop {
            match task::block_on(self.0.read_line(&mut line)) {
                Ok(n) if n > 0 => {
                    if let Some(pair) = parse_line(line.trim_end()) {
                        break Some(pair);
                    }
                    line.clear();
                }
                _ => break None,
            }
        }
    }
}

/// 从命令行参数解析配置文件地址
async fn path_from_args(n: usize) -> PathBuf {
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
