nginx -c $(pwd)/nginx.conf &

edinburgh-frame-forwarder &

edinburgh-ensemble-directory --scan edi-ch.digris.net:8851-8866 --scan edi-fr.digris.net:8851-8852 --scan edi-fr.digris.net:8854-8880 --scan edi-uk.digris.net:8851-8859 &

wait