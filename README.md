# gogw
a simple vpn.

# usage
```
# node(192.168.100.18) setup
/gogw -l 0.0.0.0:8080 -p 192.168.100.159:8080 &
ip addr add 10.0.0.1/24 dev gw0
# another(192.168.100.159) node setup
/gogw -l 0.0.0.0:8080 -p 192.168.100.18:8080 &
ip addr add 10.0.0.2/24 dev gw0
# okey
ping 10.0.0.1
ping 10.0.0.2
```
