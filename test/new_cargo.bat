@echo off
cd C:\Users\rwkoo\Desktop\codec\Rust\new_init
cargo run
set /p projname="Confirm project name:"
echo %projname%
cd C:\Users\rwkoo\Desktop\codec\Rust\%projname%
cargo init
code .
pause