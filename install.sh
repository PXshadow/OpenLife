echo "installing libs"
haxelib install openfl 8.9.5
haxelib install lime 7.6.3
haxelib git actuate https://github.com/jgranick/actuate
haxelib git hscript https://github.com/HaxeFoundation/hscript
haxelib git format https://github.com/haxefoundation/format
echo "setting up lime"
haxelib run lime setup
echo "test neko"
lime test neko
echo "finished"
sleep 2