nginx -c $(pwd)/nginx.conf &

edinburgh-frame-forwarder &

edinburgh-ensemble-directory --scan edi-proxy-1.digris.net:8101-8104 --scan edi-proxy-1.digris.net:8111-8114 &

wait