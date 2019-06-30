package client;
import haxe.crypto.Hmac;
import haxe.io.Bytes;
class Login
{
    var client:Client;
    //login info
    var username:String = "";
    var password:String = "";
    var challenge:String = "";
    var key:String = "";
    var index:Int = 0;
    public function new(client:Client)
    {
        this.client = client;
        //set test user
        username = "test@test.co.uk";
        password = "WC2TM-KZ2FP-LW5A5-LKGLP";
    }
    public function message(data:String) {
        switch(client.tag)
        {
            case SERVER_INFO:
			switch(index)
			{
				case 0:
				//current
				case 1:
				//challenge
				challenge = data;
				case 2: 
				//version
				//version = data;
                trace("get version");
                client.tag = "";
			}
			index++;
            case ACCEPTED:
            trace("accept");
            client.tag = "";
            case REJECTED:
            trace("reject");
            default:
        }
    }
    public function request()
    {
		key = StringTools.replace(key,"-","");
        client.send("LOGIN " + username + " " +
		new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f")
		,Bytes.ofString(challenge,RawNative)).toHex() + " " +
		new Hmac(SHA1).make(Bytes.ofString(key)
		,Bytes.ofString(challenge)).toHex() +  " " +
        //tutorial 1 = true 0 = false
        1 + " " +
        //twin extra code
        ""
        );
		client.tag = "";
    }
}