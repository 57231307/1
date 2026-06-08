import sys
content = open('src/main.rs').read()

start_idx = content.find('    let http_addr: SocketAddr =')
if start_idx != -1:
    new_tail = """    let http_addr: SocketAddr =
        format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    info!("HTTP 服务器监听地址：{}", http_addr);

    info!("===========================================");
    info!("系统启动完成，等待请求...");
    info!("HTTP 地址: {}", http_addr);
    info!("===========================================");

    let http_server = axum::serve(tokio::net::TcpListener::bind(http_addr).await?, app)
        .with_graceful_shutdown(async {
            shutdown_signal().await;
        });

    if let Err(e) = http_server.await {
        warn!("HTTP 服务器错误: {}", e);
    }

    Ok(())
}"""
    content = content[:start_idx] + new_tail
    open('src/main.rs', 'w').write(content)
