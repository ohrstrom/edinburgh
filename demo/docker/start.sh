nginx -c $(pwd)/nginx.conf &

edinburgh-frame-forwarder &

edinburgh-ensemble-directory --scan edi-ch.digris.net:8851-8865 &

wait