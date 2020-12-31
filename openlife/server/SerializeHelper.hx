package openlife.server;

import sys.io.File;
import openlife.data.object.ObjectData;
import openlife.settings.ServerSettings;

class SerializeHelper
{
    public static function createReadWriteFile()
    {
        var rtti = haxe.rtti.Rtti.getRtti(ObjectData);
        var dir = './${ServerSettings.SaveDirectory}/';
        var writer = File.write(dir + "writer.txt", false);
        var writer2 = File.write(dir + "reader.txt", false);

        var count = 0;
        
        for (field in rtti.fields)
        {
            count++;

            switch('${field.type}')
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

                default:
                    trace('${field.name} $count ${field.type}');
            }           
        }

        writer.close();
        writer2.close();
    }
}
