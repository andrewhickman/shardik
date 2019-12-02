cargo build --release

$env:RUST_LOG="server,client"
$Target = Join-Path $PSScriptRoot "target/release"

Start-Process cmd -ArgumentList "/c $Target/server.exe --shard-count 1 --item-count 4096"

Start-Sleep -Seconds 0.1

Start-Process cmd -ArgumentList "/c $Target/client.exe --client-name client-1 --tui --shard-count 1 --item-count 4096"
Start-Process cmd -ArgumentList "/c $Target/client.exe --client-name client-2 --tui --shard-count 1 --item-count 4096"
Start-Process cmd -ArgumentList "/c $Target/client.exe --client-name client-3 --tui --shard-count 1 --item-count 4096"