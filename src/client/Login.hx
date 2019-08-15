package client;
import haxe.crypto.Hmac;
import haxe.io.Bytes;
class Login
{
    //login info
    public var email:String = "";
    public var challenge:String = "";
    public var key:String = "";
    public var twin:String = "";
    public var tutorial:Bool = false;
    var index:Int = 0;
    public var version:Int = 0;
    //functions
    public var accept:Void->Void;
    public var reject:Void->Void;
    public function new()
    {
        
    }
    public function message(data:String) 
    {
        //login process
        switch(Main.client.tag)
        {
            case SERVER_INFO:
			switch(index)
			{
				case 0:
				//current
                trace("amount " + data);
				case 1:
				//challenge
				challenge = data;
				case 2: 
				//version
				version = Std.parseInt(data);
                request();
                Main.client.tag = "";
			}
			index++;
            default:
        }
    }
    private function request()
    {
		key = StringTools.replace(key,"-","");
        Main.client.send("LOGIN " + email + " " +

		new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f")
		,Bytes.ofString(challenge,RawNative)).toHex() + " " +

		new Hmac(SHA1).make(Bytes.ofString(key)
		,Bytes.ofString(challenge)).toHex() +  " " +

        //tutorial 1 = true 0 = false
        (tutorial ? 1 : 0) + " " +
        //twin extra code
        twin
        );
		Main.client.tag = "";
        trace("send login request");
    }
}