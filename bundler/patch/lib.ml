let main_lib () = 
    Callback.register "StoreID.initialize" StoreID.initialize ;
    Callback.register "StoreID.equal" StoreID.equal ;
    Callback.register "StoreID.compare" StoreID.compare ;
    Callback.register "StoreID.hash" StoreID.hash ;
    Callback.register "StoreID.fresh" StoreID.fresh ;
    Callback.register "StoreID.show_direct" StoreID.show_direct
