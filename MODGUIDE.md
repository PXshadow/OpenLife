```haxe
class Main extends game.Game
{
    public function new()
    {
        directory(); //gives bool of whether the local directory was valid
        super(); //creates client, data, and settings (settings and data uses directory)

        //setup client
        client.ip = "ipadress";
        client.port = 8005;
        client.email = "steam@steamgames.com";
        client.key = "FFF-FFF-FFF";

        connect(); //connect the client and attempt to login
    }
    
    override public function update(_)
    {
        super.update(null);
        //update logic
    }
}
```