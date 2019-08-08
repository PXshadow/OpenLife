package client;
@:enum abstract MessageTag(Null<String>)
{
public var COMPRESSED_MESSAGE = "CM";
public var MAP_CHUNK  = "MC"; 
public var PLAYER_UPDATE  = "PU";
public var PLAYER_MOVES_START = "PM";
public var PLAYER_OUT_OF_RANGE = "PO";
public var PLAYER_SAYS = "PS";
public var LOCATION_SAYS = "LS";
public var PLAYER_EMOT = "PE";
public var MAP_CHANGE = "MX";
public var FOOD_CHANGE = "FX";
public var HEAT_CHANGE = "HX";
public var LINEAGE = "LN";
public var NAME = "NM";
public var APOCALYPSE = "AP";
public var APOCALYPSE_DONE = "AD";
public var DYING = "DY";
public var HEALED = "HE";
public var MONUMENT_CALL = "MN";
public var GRAVE = "GV";
public var GRAVE_MOVE = "GM";
public var GRAVE_OLD = "GO";
public var OWNER_LIST = "OW";
public var VALLEY_SPACING = "VS";
public var CURSED = "CU";
public var CURSE_TOKEN_CHANGE = "CX";
public var CURSE_SCORE_CHANGE = "CS";
public var FLIGHT_DEST = "FD";
public var VOG_UPDATE = "VU";
public var PHOTO_SIGNATURE = "PH";
public var FORCED_SHUTDOWN = "SD";
public var FRAME = "FM";
public var PONG = "PONG";
//new
public var ACCEPTED = "ACCEPTED";
public var REJECTED = "REJECTED";
public var SERVER_INFO = "SN";

@:from private static function fromString(value:String):MessageTag
{
    //trace("set tag " + value);
	return switch (value)
	{
		case "CM": COMPRESSED_MESSAGE;
		case "MC": MAP_CHUNK;
		case "PU": PLAYER_UPDATE;
		case "PM": PLAYER_MOVES_START;
		case "PO": PLAYER_OUT_OF_RANGE;
		case "PS": PLAYER_SAYS;
        case "LS": LOCATION_SAYS;
        case "PE": PLAYER_EMOT;
        case "MX": MAP_CHANGE;
        case "FX": FOOD_CHANGE;
        case "HX": HEAT_CHANGE;
        case "LN": LINEAGE;
        case "NM": NAME;
        case "AP": APOCALYPSE;
        case "AD": APOCALYPSE_DONE;
        case "DY": DYING;
        case "HE": HEALED;
        case "MN": MONUMENT_CALL;
        case "GV": GRAVE;
        case "GM": GRAVE_MOVE;
        case "GO": GRAVE_OLD;
        case "OW": OWNER_LIST;
        case "VS": VALLEY_SPACING;
        case "CU": CURSED;
        case "CX": CURSE_TOKEN_CHANGE;
        case "CS": CURSE_SCORE_CHANGE;
        case "FD": FLIGHT_DEST;
        case "VU": VOG_UPDATE;
        case "PH": PHOTO_SIGNATURE;
        case "SD": FORCED_SHUTDOWN;
        case "FM": FRAME;
        case "PONG": PONG;
        //new
        case "SN": SERVER_INFO;
        case "ACCEPTED": ACCEPTED;
        case "REJECTED": REJECTED;
		default: null;
	}
}

}