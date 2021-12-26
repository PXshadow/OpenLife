package openlife.server;

class PlayerAccount
{
    public static var AllPlayerAccounts = new Map<String, PlayerAccount>();
    
    public var lastConnection:Connection;

    public var name:String;
    public var account_key_hash:String;
    public var email:String;

    public var score:Float;
    public var femaleScore:Float; 
    public var maleScore:Float; 

    private function new(){}

    public static function GetOrCreatePlayerAccount(email:String, account_key_hash:String) : PlayerAccount
    {
        var account = AllPlayerAccounts[email];
        if(account != null) return account;

        account = new PlayerAccount();
        account.email = email;
        account.account_key_hash = account_key_hash;

        AllPlayerAccounts[account.email] = account;

        trace('New account: $email');

        return account;
    }
}