import React from "react";
export const UserContext = React.createContext({
    user: undefined,
    setUser: name => {},
  });