echo "start"
haxe neko.hxml
haxe server.hxml
cd bin
cd server
start powershell neko server.n
cd ..
cd neko
neko lib.n
sleep 10
