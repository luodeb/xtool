use anyhow::Result;
use log::info;
use std::path::PathBuf;
use tftpd::{Config, Server};

pub async fn run(port: u16, path: PathBuf) -> Result<()> {
    // 将路径转换为绝对路径
    let absolute_path = path.canonicalize()?;
    
    // 验证路径
    if !absolute_path.exists() {
        anyhow::bail!("Path does not exist: {}", absolute_path.display());
    }

    if !absolute_path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", absolute_path.display());
    }

    info!("TFTP server starting on 0.0.0.0:{}", port);
    info!("Serving files from: {}", absolute_path.display());

    // 构建 tftpd 的命令行参数
    // tftpd Config 期望的参数格式: [program_name, "-i", "ip", "-p", "port", "-d", "directory"]
    let args = vec![
        "xtool".to_string(),
        "-i".to_string(),
        "0.0.0.0".to_string(),
        "-p".to_string(),
        port.to_string(),
        "-d".to_string(),
        absolute_path.to_string_lossy().to_string(),
    ];

    // 创建配置
    let config = Config::new(args.into_iter())
        .map_err(|e| anyhow::anyhow!("Failed to create TFTP config: {}", e))?;
    
    // 创建并启动服务器
    let mut server = Server::new(&config)
        .map_err(|e| anyhow::anyhow!("Failed to create TFTP server: {}", e))?;
    
    // listen() 是阻塞调用，会一直运行直到被终止
    server.listen();
    
    Ok(())
}
