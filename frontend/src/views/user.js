import React from "react";
import Container from "react-bootstrap/Container";
import Col from "react-bootstrap/Col";
import Row from "react-bootstrap/Row";
import ButtonToolbar from "react-bootstrap/ButtonToolbar";
import Modal from "react-bootstrap/Modal";
import Badge from "react-bootstrap/Badge";
import ListGroup from "react-bootstrap/ListGroup";
import Button from "react-bootstrap/Button";
import Error from "../components/error";
import Form from "react-bootstrap/Form";
import Alert from "react-bootstrap/Alert";
import { api_services_user, api_get_user_info, api_get_perms, Permissions, api_set_perms, api_set_user_info, api_delete_user, api_totp_change, api_password_change_admin } from "../lib/Api";

function ServiceEntry (props) {
    let badge = null;
    if (props.val.has_perm) {
        badge = <Badge variant="primary">Has Permissions</Badge>;
    }
    return (
        <React.Fragment>
            {props.val.name} {badge}
        </React.Fragment>
    );
}

export default class User extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            services: [],
            dialog_permission: undefined,
            name: "<Loading>",
            email: "<Loading>",
            error: undefined,
            dialog_service: undefined,
            error_store: undefined,
            storing_perms: false,
            storing_user: false,
            dialog_delete: false,
            dialog_totp: false,
            dialog_password: false,
            loading_delete: false,
            loading_totp: false,
            loading_password: false,
            new_password: '',
            new_password_repeat: '',
            password_mismatch: false,
        }

        this.handleInputChange = this.handleInputChange.bind(this);
        this.showPermissions = this.showPermissions.bind(this);
        this.showDelete = this.showDelete.bind(this);
        this.hidePermissions = this.hidePermissions.bind(this);
        this.hideDelete = this.hideDelete.bind(this);
        this.setPermission = this.setPermission.bind(this);
        this.savePermissions = this.savePermissions.bind(this);
        this.saveUserData = this.saveUserData.bind(this);
        this.loadUserInfo = this.loadUserInfo.bind(this);
        this.deleteUser = this.deleteUser.bind(this);
        this.hidePassword = this.hidePassword.bind(this);
        this.hideTOTP = this.hideTOTP.bind(this);
        this.showPassword = this.showPassword.bind(this);
        this.showTOTP = this.showTOTP.bind(this);
        this.resetTOTP = this.resetTOTP.bind(this);
        this.changePassword = this.changePassword.bind(this);
    }

    saveUserData (event) {
        event.preventDefault();
        const user = this.getUID();
        const email = this.state.email;
        const name = this.state.name;
        api_set_user_info(user, name, email)
            .then(() => {
                this.setState({ storing_user: false, error: undefined });
            })
            .catch(err => {
                if (err.response && err.response.status === 409) {
                    this.setState({ error: "Email is already in use." });
                } else {
                    this.setState({ error: "Error updating user: " + err });
                }
            })
    }

    savePermissions () {
        this.setState({ storing_perms: true });
        api_set_perms(this.getUID(), this.state.dialog_service, this.state.dialog_permission)
            .then(() => {
                this.setState({ dialog_permission: undefined, dialog_service: undefined });
            })
            .catch(err => {
                this.setState({ error_store: "Unable to store changes: " + err });
            })
            .then(() => {
                this.setState({ storing_perms: false });
            })
    }

    hidePermissions () {
        this.setState({ dialog_service: undefined });
    }

    showPermissions (service) {
        api_get_perms(this.getUID(), service)
            .then(res => {
                this.setState({ error: undefined, dialog_permission: res.data.perms, dialog_service: service, error_store: undefined, storing_perms: false });
            })
            .catch(err => {
                this.setState({ error: "Unable to fetch permissions " + err });
            })
    }

    hideDelete () {
        this.setState({dialog_delete: false});
    }

    showDelete () {
        this.setState({dialog_delete: true});
    }

    showTOTP () {
        this.setState({dialog_totp: true});
    }

    hideTOTP () {
        this.setState({dialog_totp: false});
    }

    showPassword () {
        this.setState({dialog_password: true});
    }

    hidePassword () {
        this.setState({dialog_password: false});
    }

    setPermission (event) {
        let value = event.target.checked;
        let flag = Number(event.target.attributes.flag.value);
        let perm = this.state.dialog_permission;
        if (value) {
            perm = perm | flag;
        } else {
            perm = perm ^ flag;
        }
        this.setState({ dialog_permission: perm });
    }

    getUID () {
        return this.props.match.params.user;
    }

    handleInputChange (event) {
        const target = event.target;
        const value = target.type === 'checkbox' ? target.checked : target.value;
        const name = target.name;

        this.setState({
            [name]: value
        });
    }

    loadUserInfo () {
        api_get_user_info(this.getUID())
            .then(res => {
                this.setState({ name: res.data.name, error: undefined, email: res.data.email });
            })
            .catch(err => {
                this.setState({ error: "Unable to fetch user info: " + err });
            })
    }

    deleteUser () {
        this.setState({loading_delete: true});
        api_delete_user(this.getUID())
            .then(() => {
                this.props.history.push('/users');
            })
            .catch(err => {
                this.setState({error: "Unable to delete user: "+err, loading_delete: false});
            });
    }

    resetTOTP () {
        this.setState({loading_totp: true});
        api_totp_change(this.getUID())
            .then(() => {
                this.setState({loading_totp: false, dialog_totp: false});
            })
            .catch(err => {
                this.setState({error: "Unable to reset TOTP: "+err, loading_totp: false});
            });
    }

    changePassword (event) {
        event.preventDefault();
        if ( this.state.new_password !== this.state.new_password_repeat ) {
            this.setState({password_mismatch: true});
            return;
        }
        this.setState({loading_password: true});
        api_password_change_admin(this.getUID(), this.state.new_password)
            .then(() => {
                this.setState({loading_password: false, dialog_password: false, password_mismatch: false});
            })
            .catch(err => {
                this.setState({error: "Unable to reset TOTP: "+err, loading_password: false});
            });
    }

    componentDidMount () {
        api_services_user(this.getUID())
            .then(res => {
                this.setState({ services: res.data, error: undefined });
            })
            .catch(err => {
                this.setState({ error: "Unable to fetch users: " + err });
            })
        this.loadUserInfo();
    }

    render () {
        let showPerm = this.showPermissions;
        let services = Object.keys(this.state.services).map(function (key, index) {
            return (<ListGroup.Item flag={Permissions.START} key={this[key].id} onClick={() => showPerm(this[key].id)} action
                className="d-flex justify-content-between align-items-center">
                <ServiceEntry val={this[key]} />
            </ListGroup.Item>);
        }, this.state.services);

        let name = null;
        if (this.state.dialog_service !== undefined) {
            name = this.state.services[this.state.dialog_service].name;
        }

        let button_perm_name = "Save changes";
        if (this.state.storing_perms) {
            button_perm_name = "Saving..";
        }

        let button_user_name = "Save changes";
        if (this.state.storing_user) {
            button_user_name = "Saving..";
        }

        let button_delete_name = "Delete User";
        if (this.state.loading_delete) {
            button_delete_name = "Deleting..";
        }

        let button_totp_name = "Reset TOTP";
        if (this.state.loading_totp) {
            button_totp_name = "Resetting..";
        }

        let button_password_name = "Change password";
        if (this.state.loding_password) {
            button_password_name = "Setting password..";
        }

        const perms = this.state.dialog_permission;

        return (<Container>
            <Error error={this.state.error} />
            <Modal show={this.state.dialog_password} onHide={this.hidePassword}>
                <Modal.Header closeButton>
                    <Modal.Title>Change password of "{this.state.name}"</Modal.Title>
                </Modal.Header>
                
                <Form onSubmit={this.changePassword}>
                    <Modal.Body>
                        { this.state.password_mismatch && (
                            <Alert variant="danger">Passwords mismatching</Alert>
                        ) }
                        <p>Please specify a new password for this user:</p>
                        <Form.Group controlId="formGroupPassword">
                            <Form.Label>Password</Form.Label>
                            <Form.Control required type="password" name="new_password" placeholder="Enter password" value={this.state.new_password} onChange={this.handleInputChange} />
                        </Form.Group>
                        <Form.Group controlId="formGroupPassword">
                            <Form.Label>Repeat password</Form.Label>
                            <Form.Control required type="password" name="new_password_repeat" placeholder="Enter password" value={this.state.new_password_repeat} onChange={this.handleInputChange} />
                        </Form.Group>
                    </Modal.Body>

                    <Modal.Footer>
                        <Button onClick={this.hidePassword} variant="secondary">Cancel</Button>
                        <Button variant="danger" type="submit" disabled={this.state.loading_password} >{button_password_name}</Button>
                    </Modal.Footer>
                </Form>
            </Modal>
            <Modal show={this.state.dialog_totp} onHide={this.hideTOTP}>
                <Modal.Header closeButton>
                    <Modal.Title>Reset TOTP of "{this.state.name}"</Modal.Title>
                </Modal.Header>

                <Modal.Body>
                    <p>Do you really want to reset TOTP for ?</p>
                    This requires a re-login to take effect for the user.
                </Modal.Body>

                <Modal.Footer>
                    <Button onClick={this.hideTOTP} variant="secondary">Cancel</Button>
                    <Button onClick={this.resetTOTP} variant="danger" disabled={this.state.loading_totp} >{button_totp_name}</Button>
                </Modal.Footer>
            </Modal>
            <Modal show={this.state.dialog_delete} onHide={this.hideDelete}>
                <Modal.Header closeButton>
                    <Modal.Title>Delete "{this.state.name}"</Modal.Title>
                </Modal.Header>

                <Modal.Body>
                    <p>Do you really want to delete this user ?</p>
                </Modal.Body>

                <Modal.Footer>
                    <Button onClick={this.hideDelete} variant="secondary">Cancel</Button>
                    <Button onClick={this.deleteUser} variant="danger" disabled={this.state.loading_delete} >{button_delete_name}</Button>
                </Modal.Footer>
            </Modal>
            <Modal show={this.state.dialog_service !== undefined} onHide={this.hidePermissions}>
                <Modal.Header closeButton>
                    <Modal.Title>Permissions on "{name}"</Modal.Title>
                </Modal.Header>

                <Modal.Body>
                    <Error error={this.state.error_store} />
                    <p>Permissions:</p>
                    <Form>
                        <Form.Check type="checkbox"
                            checked={Permissions.hasFlag(perms, Permissions.START)}
                            flag={Permissions.START} onChange={this.setPermission} label="Start service" />
                        <Form.Check type="checkbox"
                            checked={Permissions.hasFlag(perms, Permissions.STOP)}
                            flag={Permissions.STOP} onChange={this.setPermission} label="Stop service" />
                        <Form.Check type="checkbox"
                            checked={Permissions.hasFlag(perms, Permissions.KILL)}
                            flag={Permissions.KILL} onChange={this.setPermission} label="Kill service" />
                        <Form.Check type="checkbox"
                            checked={Permissions.hasFlag(perms, Permissions.STDIN_ALL)}
                            flag={Permissions.STDIN_ALL} onChange={this.setPermission} label="Stdin input" />
                        <Form.Check type="checkbox"
                            checked={Permissions.hasFlag(perms, Permissions.OUTPUT)}
                            flag={Permissions.OUTPUT} onChange={this.setPermission} label="Stdout inspect" />
                    </Form>
                </Modal.Body>

                <Modal.Footer>
                    <Button onClick={this.hidePermissions} variant="secondary">Cancel</Button>
                    <Button onClick={this.savePermissions} variant="primary" disabled={this.state.storing_perms} >{button_perm_name}</Button>
                </Modal.Footer>
            </Modal>
            <Row><h3>User Info of {this.state.name}</h3></Row>
            <Form onSubmit={this.saveUserData}>
                <Form.Group as={Row}>
                    <Form.Label column sm="2">
                        Email
                    </Form.Label>
                    <Col sm="10">
                        <Form.Control required type="email" name="email" onChange={this.handleInputChange} value={this.state.email} />
                    </Col>
                </Form.Group>
                <Form.Group as={Row}>
                    <Form.Label column sm="2">
                        Name
                    </Form.Label>
                    <Col sm="10">
                        <Form.Control required type="text" name="name" onChange={this.handleInputChange} value={this.state.name} />
                    </Col>
                </Form.Group>
                <Form.Group>
                    <ButtonToolbar>
                        <Button onClick={this.loadUserInfo} variant="secondary">Reset</Button>
                        <Button className="ml-2" variant="primary" type="submit" disabled={this.state.storing_user} >{button_user_name}</Button>
                    </ButtonToolbar>
                </Form.Group>
            </Form>
            <Row><h3>Authentication</h3></Row>
            <Row>
                <ButtonToolbar>
                    <Button onClick={this.showTOTP} variant="warning">Reset TOTP</Button>
                    <Button onClick={this.showPassword} className="ml-2" variant="warning">Change Password</Button>
                </ButtonToolbar>
            </Row>
            <Row><h3>Permissions of {this.state.name}</h3></Row>
            <Container><ListGroup>{services}</ListGroup></Container>
            <hr />
            <Row><Col><Button onClick={this.showDelete} variant="danger">Delete User</Button></Col></Row>
        </Container>)
    }
}