create table monitoring_users(
    user_id integer PRIMARY KEY
);

create table currency_minimum(
    id integer PRIMARY KEY AUTOINCREMENT,
    user_id integer,
    bank_name varchar(16),
    min_value integer,
    cur_type varchar(8),
    update_time integer,
    
    FOREIGN KEY(user_id) 
        REFERENCES monitoring_users(user_id) 
        ON DELETE CASCADE
);