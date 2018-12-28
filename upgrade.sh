git pull && cargo build && sudo systemctl stop ttdash && cp target/debug/ttdash /home/pi/ttdash && sudo systemctl start ttdash && sudo journalctl -f -u ttdash
