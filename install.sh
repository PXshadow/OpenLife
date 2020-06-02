echo "installing lib"
haxelib git format https://github.com/haxefoundation/format
echo "install libs for targets"
haxelib install hxjava
haxelib install hxnodejs
haxelib install hxcs
echo "setting up lime"
haxelib run lime setup
echo "test neko"
lime test neko
echo "finished"
sleep 2