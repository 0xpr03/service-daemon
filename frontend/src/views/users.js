import React from "react";
import Container from "react-bootstrap/Container";
import Row from "react-bootstrap/Row";
import Table from "react-bootstrap/Table";
import Col from "react-bootstrap/Col";
import Button from "react-bootstrap/Button";
import Error from "../components/error";
import { Link } from "react-router-dom";
import { api_users } from "../lib/Api";

function User (props) {
    return (
        <tr>
            <td><Link to={"/user/" + props.user.id}>{props.user.id}</Link></td>
            <td><Link to={"/user/" + props.user.id}>{props.user.name}</Link></td>
            <td><Link to={"/user/" + props.user.id}>{props.user.email}</Link></td>
        </tr>
    );
}

export default class Users extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            users: [],
            error: undefined,
        }
    }

    componentDidMount () {
        api_users()
            .then(res => {
                this.setState({ users: res.data });
            })
            .catch(err => {
                this.setState({ error: "Unable to fetch users: " + err });
            })
    }

    render () {
        let users = this.state.users.map(user =>
            <User key={user.id} user={user} />);
        return (<Container>
            <Error error={this.state.error} />
            <Table striped bordered hover>
                <thead>
                    <tr>
                        <th>#</th>
                        <th>Name</th>
                        <th>Email</th>
                    </tr>
                </thead>
                <tbody>
                    {users}
                </tbody>
            </Table>
            <Row><Col><Button as={Link} to="/new/user">Create New User</Button></Col></Row>
        </Container>
        );
    }
}