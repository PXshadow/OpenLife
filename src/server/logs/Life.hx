package server.logs;

import sys.db.Types.SBool;
import sys.db.Types.SInt;
import sys.db.Types.SId;
import sys.db.Object;
import sys.db.Types.STinyUInt;
import sys.db.Types.SMediumUInt;


class Life extends Object
{
    public var id:SId;
    public var pid:SMediumUInt;
    public var x:SInt;
    public var y:SInt;
    public var male:SBool;
    public var birth:SBool;
    public var disconnect:SBool;
}