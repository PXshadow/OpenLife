package server;
//tags used by the server
@:enum abstract ServerTag(Null<String>)
{
    public var KA = "KA";
    public var USE = "USE";
    public var BABY = "BABY";
    public var SELF = "SELF";
    public var UBABY = "UBABY";
    public var REMV = "REMV";
    public var SREMV = "SREMV";
    public var DROP = "DROP";
    public var KILL = "KILL";
    public var JUMP = "JUMP";
    public var EMOT = "EMOT";
    public var DIE = "DIE";
    public var GRAVE = "GRAVE";
    public var OWNER = "OWNER";
    public var FORCE = "FORCE";
    public var PING = "PING";
    //voice of god tags
    public var VOGS = "VOGS";
    public var VOGN = "VOGN";
    public var VOGP = "VOGP";
    public var VOGM = "VOGM";
    public var VOGI = "VOGI";
    public var VOGT = "VOGT";
    public var VOGX = "VOGX";
    /**
     * photo to scary to use
     */
    public var PHOTO = "PHOTO";
    /**
     * say for messaging
     */
    public var SAY = "SAY";
    /**
     * login
     */
    public var LOGIN = "LOGIN";
    public var RELOGIN = "RLOGIN";

    @:from private static function fromString(value:String):ServerTag
    {
        return switch (value)
        {
            case "KA": KA;
            case "USE": USE;
            case "BABY": BABY;
            case "SELF": SELF;
            case "UBABY": UBABY;
            case "REMV": REMV;
            case "SREMV": SREMV;
            case "DROP": DROP;
            case "KILL": KILL;
            case "JUMP": JUMP;
            case "EMOT": EMOT;
            case "DIE": DIE;
            case "GRAVE": GRAVE;
            case "OWNER": OWNER;
            case "FORCE": FORCE;
            case "PING": PING;
            case "VOGS": VOGS;
            case "VOGN": VOGN;
            case "VOGP": VOGP;
            case "VOGM": VOGM;
            case "VOGI": VOGI;
            case "VOGT": VOGT;
            case "VOGX": VOGX;
            case "PHOTO": PHOTO;
            case "SAY": SAY;
            case "LOGIN": LOGIN;
            default: null;
        }
    }
}