package client;
import haxe.crypto.Hmac;
import haxe.io.Bytes;
import haxe.crypto.Sha1;
class Login
{
    //login info
    public var email:String = "";
    public var challenge:String = "";
    public var key:String = "";
    public var twin:String = "coding";
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
        //login
        var login = /*"client_shadow" +*/ email + " " +
		new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f")
		,Bytes.ofString(challenge,RawNative)).toHex() + " " +
		new Hmac(SHA1).make(Bytes.ofString(key)
		,Bytes.ofString(challenge)).toHex() +  " " +
        //tutorial 1 = true 0 = false
        (tutorial ? 1 : 0);
        //twin
        //login += " " + Sha1.make(Bytes.ofString(twin)).toHex() + " 1";

        Main.client.send("LOGIN " + login);
		Main.client.tag = "";
        trace("send login request");
    }
}