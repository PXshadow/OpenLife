echo "installing lib"
haxelib git format https://github.com/haxefoundation/format
echo "install libs for targets"
haxelib install hxjava
haxelib install hxnodejs
haxelib install hxcs
echo "downloading data"
haxe setup_data.hxml
echo "test app"
haxe app.hxml
echo "finished"
sleep 2