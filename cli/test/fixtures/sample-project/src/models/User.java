package models;

import java.util.List;
import java.util.Optional;
import java.io.Serializable;

public class User implements Serializable {
    private String name;
    private String email;

    public User(String name, String email) {
        this.name = name;
        this.email = email;
    }

    public String getName() {
        return name;
    }

    public void setEmail(String email) {
        this.email = email;
    }

    private boolean validate() {
        return name != null && email != null;
    }
}

interface UserService {
    Optional<User> findById(long id);
    List<User> findAll();
}

enum UserRole {
    ADMIN,
    USER,
    GUEST
}
