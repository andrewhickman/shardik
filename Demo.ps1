cargo build --release

$env:RUST_LOG="server,client"
$Target = Join-Path $PSScriptRoot "target/release"

Start-Process cmd -ArgumentList "/c $Target/server.exe --latency 5000"

Start-Sleep -Seconds 0.1

Start-Process cmd -ArgumentList "/c $Target/client.exe --client-name normal-client --initial-key 0/0 --tui --perturb-shard-chance 0.2"
Start-Process cmd -ArgumentList "/c $Target/client.exe --client-name bad-client --initial-key 8/0 --tui --perturb-shard-chance 0.9"
Start-Process cmd -ArgumentList "/c $Target/client.exe --client-name good-client --initial-key 16/0 --tui --perturb-shard-chance 0.0"