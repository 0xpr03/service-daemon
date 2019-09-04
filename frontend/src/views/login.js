import React from "react";
import Error from "../components/error";
import Form from "react-bootstrap/Form";
import Button from "react-bootstrap/Button";
import Container from "react-bootstrap/Container";
import Row from "react-bootstrap/Row";
import Col from "react-bootstrap/Col"
import LoadingButton from "../components/loading-button";
import Alert from "react-bootstrap/Alert";
import QRCode from 'qrcode.react';
import { api_login, api_totp, api_checklogin, AuthState } from "../lib/Api";
import {
    Redirect,
} from "react-router-dom";
import Loading from "../components/loading";
import { UserContext } from '../user-context';


const Mode = {
    INIT: -1,
    LOGGED_IN: 0,
    PASSWORD: 1,
    TOTP: 2,
    SETUP_TOTP: 3,
};

export default class Login extends React.Component {
    constructor(props) {
        console.log(props);
        super(props);
        this.state = {
            redirectToReferrer: false,
            mode: Mode.INIT,
            email: "",
            password: "",
            totp: "",
            qrcode: "",
            invalidLogin: false,
            invalidTotp: false,
            error: undefined,
            loading: true,
        };

        this.handleInputChange = this.handleInputChange.bind(this);
        this.handleSubmitLogin = this.handleSubmitLogin.bind(this);
        this.handleSubmitTotp = this.handleSubmitTotp.bind(this);
    }


    componentDidMount () {
        api_checklogin()
            .then(response => {
                this.updateMode(response);
            })
            .catch(error => {
                console.error(error);
                this.setError("Unable to fetch current login: " + error);
                this.setLoading(false);
            });
    }

    clearError () {
        this.setError(undefined);
    }
    setError (err) {
        this.setState({ error: err });
    }
    setLoading (loading) {
        console.log("loading: "+loading);
        this.setState({ loading });
    }

    handleInputChange (event) {
        const target = event.target;
        const value = target.type === 'checkbox' ? target.checked : target.value;
        const name = target.name;

        this.setState({
            [name]: value
        });
    }

    login_totp () {
        let token = this.state.totp;
        this.clearError();
        this.setLoading(true);
        api_totp(token)
            .then(response => {
                this.updateMode(response);
            })
            .catch(error => {
                this.setLoading(false);
                if (error.response) {
                    if (error.response.status === 403) {
                        this.setState({ invalidTotp: true });
                    }
                } else {
                    console.error(error);
                    this.setError("Unable to login via TOTP: " + error);
                }
            });
    }

    updateMode (response) {
        let r = response.data;
        // avoid update on component unmount
        if (r[AuthState.LOGGED_IN] === undefined) {
            this.setLoading(false);
        }
        if (r[AuthState.SETUP_TOTP] !== undefined) {
            let data = response.data[AuthState.SETUP_TOTP];
            this.setState({
                qrcode: "otpauth://totp/ServiceController:" + this.email + "?secret=" + data.secret + "&issuer=SC&algorithm=" + data.mode + "&digits=" + data.digits,
                mode: Mode.SETUP_TOTP
            });
        } else if (r === AuthState.TOTP) {
            this.setState({ mode: Mode.TOTP });
        } else if (r === AuthState.PASSWORD) {
            this.setState({ mode: Mode.PASSWORD });
        } else if (r[AuthState.LOGGED_IN] !== undefined) {
            this.setState({ mode: Mode.LOGGED_IN });
            this.context.setUser(r[AuthState.LOGGED_IN]);
        } else {
            this.setState({ error: "Unknown login state!" });
            console.error(r);
        }
    }

    login_password () {
        let email = this.state.email;
        let password = this.state.password;
        this.clearError();
        this.setLoading(true);
        api_login(email, password)
            .then(response => {
                this.updateMode(response);
            })
            .catch(error => {
                this.setState({loading: false});
                if (error.response) {
                    if (error.response.status === 403) {
                        this.setState({ invalidLogin: true });
                    }
                } else {
                    console.error(error);
                    this.setLoading(false);
                    this.setError("Unable to login! " + error);
                }
            });
    }

    handleSubmitLogin (event) {
        event.preventDefault();
        console.log("on submit");
        this.login_password();
    }

    handleSubmitTotp (event) {
        event.preventDefault();
        console.log("on submit totp");
        this.login_totp();
    }

    render () {
        if (this.state.loading && this.state.mode === Mode.INIT) {
            return (<Loading />);
        } else {
            let { from } = this.props.location.state || { from: { pathname: "/" } };
            return (
                <UserContext.Consumer>
                    {({ user, setUser }) => (
                        <Container className="h-100">
                            <Error error={this.state.error} />
                            {this.state.mode === Mode.LOGGED_IN &&
                                <Redirect to={from} />}
                            {this.state.mode === Mode.PASSWORD && this.password()}
                            {this.state.mode === Mode.TOTP && this.totp()}
                            {this.state.mode === Mode.SETUP_TOTP && this.totp()}
                        </Container>
                    )}
                </UserContext.Consumer>
            );
        }
    }

    error () {
        if (this.state.error !== undefined) {
            return (<Alert variant="danger">{this.state.error}</Alert>);
        } else {
            return null;
        }
    }

    password () {
        return (
            <Row className="h-100 justify-content-center align-items-center vertical-center">
                <Form onSubmit={this.handleSubmitLogin} className="col-6">
                    {(this.state.invalidLogin) && <Alert variant="warning">
                        Invalid Login!
                    </Alert>}
                    <Form.Group controlId="formGroupEmail">
                        <Form.Label>Email address</Form.Label>
                        <Form.Control required type="email" name="email" placeholder="Enter email" value={this.state.email} onChange={this.handleInputChange} />
                    </Form.Group>
                    <Form.Group controlId="formGroupPassword">
                        <Form.Label>Password</Form.Label>
                        <Form.Control required type="password" name="password" placeholder="Enter password" value={this.state.password} onChange={this.handleInputChange} />
                    </Form.Group>
                    <LoadingButton variant="primary" type="submit" isLoading={this.state.loading}>
                        Login
                    </LoadingButton>
                </Form>
            </Row>
        );
    }

    totp () {
        let name = this.state.mode === Mode.TOTP ? "Verify" : "Setup TOTP";
        console.log("qrcode:" + this.state.qrcode);
        return (
            <div className="h-100 justify-content-center align-items-center vertical-center">
                <Col>
                {
                    (this.state.mode === Mode.SETUP_TOTP) && (
                        <><Row className="justify-content-center"><small>Please note that you can't use google authenticator due to bugs. You may use <a href="https://github.com/andOTP/andOTP#downloads">andOTP</a></small></Row>
                        <Row className="justify-content-center">
                            <QRCode value={this.state.qrcode} size={191}
                                renderAs={"svg"} includeMargin={true} />
                        </Row></>
                    )
                }
                <Row className="justify-content-center">
                    <Form onSubmit={this.handleSubmitTotp}>
                        {(this.state.invalidTotp) && <Alert variant="warning">
                            Invalid Token!
                        </Alert>}
                        <Form.Group controlId="formGroupTOTP">
                            <Form.Label>Authentication code</Form.Label>
                            <Form.Control type="text" name="totp" placeholder="Enter TOTP" value={this.state.totp} onChange={this.handleInputChange} />
                        </Form.Group>
                        <LoadingButton variant="primary" type="submit" block isLoading={this.state.loading}>
                            {name}
                        </LoadingButton>
                        <Button variant="link" href="/logout">Logout</Button>
                    </Form>
                    </Row>
                    </Col>
            </div>
        );
    }
}

Login.contextType = UserContext;