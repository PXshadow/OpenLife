echo "installing libs"
haxelib install openfl 8.9.5
haxelib install lime 7.6.3
haxelib git actuate https://github.com/jgranick/actuate
haxelib git hscript https://github.com/HaxeFoundation/hscript
haxelib git format https://github.com/haxefoundation/format
haxelibi git hxcpp-debug-server https://github.com/vshaxe/hxcpp-debugger
echo "installing libs for test-adapter"
haxelib install utest
haxelib install json2object
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