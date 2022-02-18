package openlife.server;

import openlife.data.object.player.PlayerInstance;
import sys.io.File;
import openlife.data.object.ObjectData;
import openlife.settings.ServerSettings;

class SerializeHelper
{
    public static function createReadWriteFile()
    {
        var rtti = haxe.rtti.Rtti.getRtti(GlobalPlayerInstance);
        var dir = './${ServerSettings.SaveDirectory}/';
        var loadText = '';
        var endtext = '';
        var writer = File.write(dir + "writerGlobalPlayerInstance.txt", false);
        var writer2 = File.write(dir + "readerGlobalPlayerInstance.txt", false);

        var count = 0;
        
        for (field in rtti.fields)
        {
            count++;

            var typename = '${field.type}';

            switch(typename)
            {
                case "CAbstract(Int,[])":
                    writer.writeString('\t\twriter.writeInt32(obj.${field.name});\n');
                    writer2.writeString('\t\tobj.${field.name} = reader.readInt32();\n');
                    //trace('FOUND: ${field.type.getName} ${field.type}'); // Int

                case "CAbstract(Float,[])":
                    writer.writeString('\t\twriter.writeFloat(obj.${field.name});\n');
                    writer2.writeString('\t\tobj.${field.name} = reader.readFloat();\n');

                case "CAbstract(Bool,[])":
                    writer.writeString('\t\twriter.writeInt8(obj.${field.name} ? 1 : 0);\n');
                    writer2.writeString('\t\tobj.${field.name} = reader.readInt8() != 0 ? true : false;\n');

                case "CClass(String,[])":
                    writer.writeString('\t\twriter.writeInt16(obj.${field.name}.length);\n');
                    writer.writeString('\t\twriter.writeString(obj.${field.name});\n');

                    writer2.writeString('\t\tvar len = reader.readInt16();\n');
                    writer2.writeString('\t\tobj.${field.name} = reader.readString(len);\n');

                case "CClass(Array,[CAbstract(Int,[])])":
                    writer.writeString('\t\twriter.writeInt16(obj.${field.name}.length);\n');
                    writer.writeString('\t\tfor(i in obj.${field.name}) writer.writeInt32(i);\n');

                    writer2.writeString('\t\tobj.${field.name} = new Array<Int>();\n'); 
                    writer2.writeString('\t\tvar len = reader.readInt16();\n');
                    writer2.writeString('\t\tfor(i in 0...len){obj.${field.name}[i] = reader.readInt32();}\n');
                case "CClass(openlife.server.GlobalPlayerInstance,[])":
                    writer.writeString('\t\twriter.writeInt32(GetPlayerIdForWrite(obj.${field.name})); //$count\n');
                    writer2.writeString('\t\tplayersToLoad[obj.p_id]["${field.name}"] = reader.readInt32(); //$count\n');
                    loadText += '\t\tobj.${field.name} = GetPlayerFromId(playersToLoad[obj.p_id]["${field.name}"]); //$count\n';
                case "CClass(openlife.data.object.ObjectHelper,[])":
                    writer.writeString('\t\tObjectHelper.WriteToFile(obj.${field.name}, writer); //$count\n');
                    writer2.writeString('\t\tobj.${field.name} = ObjectHelper.ReadFromFile(reader); //$count\n');
                default:
                    if(StringTools.contains(typename, 'CFunction') == false)
                    {
                        trace('FIELD: ${field.name} $count ${field.type}');
                        writer.writeString('\t\t//${field.name} $count ${field.type}\n');
                        writer2.writeString('\t\t//${field.name} $count ${field.type}\n');
                        endtext += '\t\t//${field.name} $count ${field.type}\n';
                    }
            }           
        }

        writer.writeString('\n' + endtext);
        writer2.writeString('\n' + loadText);
        
        writer.close();
        writer2.close();
    }
}
