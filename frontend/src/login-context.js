import React from 'react';

export const LoginContext = React.createContext({
    user: undefined,
    changeUser: () => { },
});