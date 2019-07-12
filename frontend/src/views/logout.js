import React from "react";
import { api_logout } from "../lib/Api";
import {
    Redirect,
} from "react-router-dom";
import { UserContext } from '../user-context';
import { Container } from 'react-bootstrap';
import Error from "../components/error";

export default class Logout extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            logged_out: false,
            error: undefined,
        };
    }
    componentDidMount () {
        api_logout()
            .then(response => {
                console.log("logout state:");
                console.log(response);
                this.context.setUser(undefined);
                this.setState({ logged_out: true });
            })
            .catch(err => {
                console.error(err);
                this.setState({ error: "Unable to logout: " + err });
            });
    }

    render () {
        return (
            <Container>
                <Error error={this.state.error} />
                {this.state.logged_out ? (
                    <Redirect to="/" />) : (
                        <h3>Logging out..</h3>
                    )}
            </Container>
        );
    }
}
Logout.contextType = UserContext;