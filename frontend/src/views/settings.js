import React from "react";
import Container from "react-bootstrap/Container";
import ButtonToolbar from "react-bootstrap/ButtonToolbar";
import Button from "react-bootstrap/Button";
import Alert from "react-bootstrap/Alert";
import Error from "../components/error";
import Form from "react-bootstrap/Form";
import { UserContext } from '../user-context';
import { api_set_user_info, api_password_change } from "../lib/Api";

export default class Settings extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            loading: true,
            name: undefined,
            email: undefined,
            error: undefined,
            old_password: "",
            new_password: "",
            new_password_repeat: "",
            storing_settings: false,
            changing_password: false,
            password_changed: false,
            password_mismatch: false,
        }

        this.saveSettings = this.saveSettings.bind(this);
        this.loadUserInfo = this.loadUserInfo.bind(this);
        this.handleInputChange = this.handleInputChange.bind(this);
        this.changePassword = this.changePassword.bind(this);
    }

    handleInputChange (event) {
        const target = event.target;
        const value = target.type === 'checkbox' ? target.checked : target.value;
        const name = target.name;

        this.setState({
            [name]: value
        });
    }

    saveSettings (event) {
        event.preventDefault();
        const name = this.state.name;
        const email = this.state.email;
        this.setState({ storing_settings: true });
        api_set_user_info(this.context.user.id, name, email)
            .then(() => {
                this.context.updateInfo(name, email);
                this.setState({ error: undefined });
            })
            .catch(err => this.setState({ error: "Unable to update user info. " + err }))
            .then(() => this.setState({storing_settings: false }));
    }

    changePassword (event) {
        event.preventDefault();
        const old_password = this.state.old_password;
        const new_password = this.state.new_password;
        const new_password_repeat = this.state.new_password_repeat;
        if (new_password !== new_password_repeat) {
            //TODO: validate form
            this.setState({ password_mismatch: true });
            return;
        }
        this.setState({ changing_password: true, password_mismatch: false, password_changed: false });
        api_password_change(this.context.user.id, new_password, old_password)
            .then(() =>
                this.setState({ error: undefined, password_changed: true, old_password: "", new_password: "", new_password_repeat: "" })
            )
            .catch(err => this.setState({ error: "Unable to change password: " + err }))
            .then(() => this.setState({changing_password: false }));
    }

    componentDidMount () {
        this.loadUserInfo();
    }

    loadUserInfo () {
        this.setState({
            name: this.context.user.name,
            email: this.context.user.email
        });
    }

    render () {
        let button_store_settings = "Save";
        if (this.storing_settings) {
            button_store_settings = "Storing...";
        }
        let button_change_password = "Change Password";
        if (this.changing_password) {
            button_change_password = "Changing...";
        }

        return (<Container className="pt-md-2">
            <div className="header mb-4"><h2>Account Settings</h2></div>
            <Error error={this.state.error} />
            <Form onSubmit={this.saveSettings}>
                <Form.Group>
                    <Form.Label className="formBold">
                        Email
                    </Form.Label>
                    <Form.Control required type="email" name="email" onChange={this.handleInputChange} value={this.state.email} />
                </Form.Group>
                <Form.Group>
                    <Form.Label className="formBold">
                        Name
                    </Form.Label>
                    <Form.Control required type="text" name="name" onChange={this.handleInputChange} value={this.state.name} />
                </Form.Group>
                <Form.Group>
                    <ButtonToolbar>
                        <Button onClick={this.loadUserInfo} variant="secondary">Reset</Button>
                        <Button className="ml-2" variant="primary" type="submit" disabled={this.state.storing_settings} >{button_store_settings}</Button>
                    </ButtonToolbar>
                </Form.Group>
            </Form>
            <Form onSubmit={this.changePassword}>
                {/* TODO: make success repeatable */}
                {this.state.password_mismatch && <Alert onClose={this.hide} dismissible variant="warn">New passwords don't match.</Alert>}
                {this.state.password_changed && <Alert onClose={this.hide} dismissible variant="success">Password changed successfully.</Alert>}
                <Form.Group>
                    <Form.Label className="formBold">
                        Old password
                    </Form.Label>
                    <Form.Control name="old_password" required type="password" onChange={this.handleInputChange} value={this.state.old_password} />
                </Form.Group>
                <Form.Group>
                    <Form.Label className="formBold">
                        New password
                    </Form.Label>
                    <Form.Control name="new_password" required type="password" onChange={this.handleInputChange} value={this.state.new_password} />
                </Form.Group>
                <Form.Group>
                    <Form.Label className="formBold">
                        Confirm new password
                    </Form.Label>
                    <Form.Control name="new_password_repeat" required type="password" onChange={this.handleInputChange} value={this.state.new_password_repeat} />
                </Form.Group>
                <Button variant="primary" disabled={this.state.changing_password} type="submit">
                    {button_change_password}
                </Button>
            </Form>
        </Container>)
    }
}

Settings.contextType = UserContext;