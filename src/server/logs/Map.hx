package server.logs;

import sys.db.Types.SNull;
import sys.db.Types.STinyInt;
import sys.db.Types.SBool;
import sys.db.Types.SInt;
import sys.db.Types.SId;
import sys.db.Object;
import sys.db.Types.STinyUInt;
import sys.db.Types.SMediumUInt;


class Map extends Object
{
    public var id:SId;
    public var floor:SInt;
    public var object:SInt;
    public var grave:SNull<SInt>;
}