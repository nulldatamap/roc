platform folkertdev/foo
    requires { main : Effect {} }
    exposes [ Task ]
    packages {}
    imports []
    provides [ mainForHost ]
    effects Effect
        {
            putChar : Int -> Effect {},
            putLine : Str -> Effect {},
            getLine : Effect Str
        }

mainForHost : Effect {} as Fx
mainForHost = main
