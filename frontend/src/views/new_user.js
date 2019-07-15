import React from "react";
import Container from "react-bootstrap/Container";
import Form from "react-bootstrap/Form";
import Button from "react-bootstrap/Button";
import Error from "../components/error";
import { api_create_user } from "../lib/Api";

export default class NewUser extends React.Component {
    constructor(props) {
        super(props)

        this.state = {
            error: undefined,
            loading: false,
            user: undefined,
            name: "",
            password: "",
            email: "",
        }

        this.onCreateUser = this.onCreateUser.bind(this);
        this.handleInputChange = this.handleInputChange.bind(this);
        this.onReset = this.onReset.bind(this);
    }

    onReset () {
        this.setState({ name: "", password: "", email: "" });
    }

    handleInputChange (event) {
        const target = event.target;
        const value = target.type === 'checkbox' ? target.checked : target.value;
        const name = target.name;

        this.setState({
            [name]: value
        });
    }

    onCreateUser (event) {
        event.preventDefault();
        this.setState({ loading: true });
        const name = this.state.name;
        const email = this.state.email;
        const password = this.state.password;
        api_create_user(name, email, password)
            .then(resp => {
                this.props.history.push('/user/' + resp.data.user);
            })
            .catch(err => {
                if (err.response && err.response.status === 409) {
                    this.setState({ error: "Email is already in use." });
                } else {
                    this.setState({ error: "Unable to create user: " + err });
                }
            })
            .then(() => {
                this.setState({ loading: false });
            });
    }

    render () {
        let button_submit_name = "Create User";
        if (this.state.loading) {
            button_submit_name = "Loading..";
        }

        return (
            <Container>
                <Error error={this.state.error} />
                <Form onSubmit={this.onCreateUser}>
                    <Form.Group controlId="formGroupEmail">
                        <Form.Label>Email address</Form.Label>
                        <Form.Control required type="email" name="email" placeholder="Email@example.com" value={this.state.email} onChange={this.handleInputChange} />
                    </Form.Group>
                    <Form.Group controlId="formGroupPassword">
                        <Form.Label>Password</Form.Label>
                        <Form.Control required type="text" name="password" placeholder="password" value={this.state.password} onChange={this.handleInputChange} />
                    </Form.Group>
                    <Form.Group controlId="formGroupName">
                        <Form.Label>Name</Form.Label>
                        <Form.Control required type="text" name="name" placeholder="Name" value={this.state.name} onChange={this.handleInputChange} />
                    </Form.Group>
                    <Button onClick={this.onReset}>Reset</Button>
                    <Button variant="primary" type="submit" disabled={this.state.loading} >{button_submit_name}</Button>
                </Form>
            </Container>
        );
    }
}