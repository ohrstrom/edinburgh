odr-dabmux /etc/odr-dabmux.json &

# damux gui does not contain static files
# /usr/local/bin/odr-dabmux-gui &
(cd /mux/ODR-DabMux-GUI && ./target/release/odr-dabmux-gui) &

wait